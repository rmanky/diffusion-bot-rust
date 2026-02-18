use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use std::time::Instant;
use tokio_stream::StreamExt;
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::Attachment as ChannelAttachment;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{
    EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::utils::embed;

use super::{CommandHandler, CommandHandlerData};

#[derive(CommandModel, CreateCommand)]
#[command(name = "chat", desc = "Chat with Kimi K2.5")]
pub struct ChatCommand {
    /// Prompt to send to the model.
    prompt: String,
    /// Optional image to send alongside the prompt.
    image: Option<ChannelAttachment>,
}

const API_URL: &str = "https://api.replicate.com/v1/models/moonshotai/kimi-k2.5/predictions";
const MAX_EMBED_LEN: usize = 4096;

struct ChatContext<'a> {
    prompt: &'a str,
    image_url: Option<&'a str>,
    interaction_client: &'a InteractionClient<'a>,
    interaction_token: &'a str,
    reqwest_client: &'a Client,
}

impl<'a> ChatContext<'a> {
    fn prompt_embed(&self) -> EmbedBuilder {
        let builder = embed::prompt(self.prompt);
        match self.image_url.and_then(|url| ImageSource::url(url).ok()) {
            Some(source) => builder.image(source),
            None => builder,
        }
    }

    async fn update_embeds(&self, prediction_id: &str, status_embed: EmbedBuilder) {
        self.interaction_client
            .update_response(self.interaction_token)
            .embeds(Some(&[
                self.prompt_embed()
                    .footer(EmbedFooterBuilder::new(prediction_id))
                    .build(),
                status_embed
                    .footer(EmbedFooterBuilder::new(prediction_id))
                    .build(),
            ]))
            .await
            .ok();
    }

    async fn run(&self) -> Result<()> {
        let submit_response = self
            .reqwest_client
            .post(API_URL)
            .header(
                "Authorization",
                format!("Bearer {}", env::var("REPLICATE_TOKEN").unwrap()),
            )
            .json(&ReplicateRequest {
                input: ReplicateInput {
                    prompt: self.prompt,
                    max_tokens: 2048,
                    image: self.image_url,
                },
                stream: true,
            })
            .send()
            .await
            .context("Failed to submit request")?
            .json::<ReplicateSubmit>()
            .await
            .context("Failed to parse submit response")?;

        let stream_url = &submit_response.urls.stream;
        let prediction_id = &submit_response.id;

        let mut es = EventSource::new(self.reqwest_client.get(stream_url))
            .context("Failed to create event source")?;

        let mut full_output = String::new();
        let mut last_update = Instant::now();
        let update_interval = Duration::from_millis(750);

        while let Some(event) = es.next().await {
            let message = match event {
                Ok(Event::Message(m)) => m,
                Err(e) => {
                    es.close();
                    if full_output.is_empty() {
                        anyhow::bail!("Stream connection error: {e:#}");
                    }
                    break;
                }
                _ => continue,
            };

            match message.event.as_str() {
                "output" => {
                    full_output.push_str(&message.data);

                    if last_update.elapsed() >= update_interval {
                        let display = truncate_display(&full_output);
                        self.update_embeds(
                            prediction_id,
                            embed::pending("Processing...", &display),
                        )
                        .await;
                        last_update = Instant::now();
                    }
                }
                "done" => {
                    es.close();
                    break;
                }
                "error" => {
                    es.close();
                    anyhow::bail!("Stream error: {}", message.data);
                }
                _ => {}
            }
        }

        let final_embed = if full_output.is_empty() {
            embed::failure("The model finished but generated no output.")
        } else {
            embed::success().description(truncate_display(&full_output))
        };

        self.update_embeds(prediction_id, final_embed).await;

        Ok(())
    }
}

#[async_trait]
impl CommandHandler for ChatCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let ctx = ChatContext {
            prompt: &self.prompt,
            image_url: self.image.as_ref().map(|img| img.url.as_str()),
            interaction_client: &command_handler_data.interaction_client,
            interaction_token,
            reqwest_client: &command_handler_data.reqwest_client,
        };

        ctx.interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![embed::pending("Chatting", "")
                            .field(EmbedFieldBuilder::new("Prompt", ctx.prompt))
                            .build()]),
                        ..Default::default()
                    }),
                }),
            )
            .await
            .ok();

        if let Err(e) = ctx.run().await {
            ctx.interaction_client
                .update_response(interaction_token)
                .embeds(Some(&[
                    ctx.prompt_embed().build(),
                    embed::failure(e).build(),
                ]))
                .await
                .ok();
        }
    }
}

#[derive(Serialize)]
struct ReplicateInput<'a> {
    prompt: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<&'a str>,
}

#[derive(Serialize)]
struct ReplicateRequest<'a> {
    input: ReplicateInput<'a>,
    stream: bool,
}

#[derive(Deserialize)]
struct Urls {
    stream: String,
}

#[derive(Deserialize)]
struct ReplicateSubmit {
    id: String,
    urls: Urls,
}

fn truncate_display(s: &str) -> String {
    if s.len() >= MAX_EMBED_LEN {
        let truncated: String = s.chars().take(MAX_EMBED_LEN - 4).collect();
        format!("{truncated}...")
    } else {
        s.to_owned()
    }
}
