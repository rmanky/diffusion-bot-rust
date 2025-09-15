use async_trait::async_trait;
use log::info;
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use std::time::Instant;
use tokio_stream::StreamExt;
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::message::Embed;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder};

use super::{CommandHandler, CommandHandlerData};

#[derive(CommandModel, CreateCommand)]
#[command(name = "chat", desc = "Chat with Snowflake Arctic")]
pub struct ChatCommand {
    /// Prompt to send to the model.
    prompt: String,
}

#[async_trait]
impl CommandHandler for ChatCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let interaction_client = command_handler_data.interaction_client;
        let reqwest_client = command_handler_data.reqwest_client;

        let prompt = &self.prompt;

        interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Chatting")
                            .color(0x673ab7)
                            .field(EmbedFieldBuilder::new("Prompt", prompt))
                            .build()]),
                        ..Default::default()
                    }),
                }),
            )
            .await
            .ok();

        let e = match chat(
            prompt,
            &reqwest_client,
            &interaction_client,
            interaction_token,
        )
        .await
        {
            Ok(_) => {
                return;
            }
            Err(e) => e,
        };

        interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[
                prompt_embed(prompt, "UNKNOWN"),
                EmbedBuilder::new()
                    .title("Failed")
                    .color(0xe53935)
                    .description(format!("```\n{}\n```", e.message))
                    .build(),
            ]))
            .await
            .ok();
    }
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

struct ChatError {
    message: String,
}

fn prompt_embed(prompt: &str, id: &str) -> Embed {
    EmbedBuilder::new()
        .title("Prompt")
        .color(0x43a047)
        .description(prompt)
        .footer(EmbedFooterBuilder::new(id))
        .build()
}

async fn chat(
    prompt: &str,
    reqwest_client: &Client,
    interaction_client: &InteractionClient<'_>,
    interaction_token: &str,
) -> Result<(), ChatError> {
    let submit_response = match reqwest_client
        .post("https://api.replicate.com/v1/models/qwen/qwen3-235b-a22b-instruct-2507/predictions")
        .header(
            "Authorization",
            format!("Bearer {}", env::var("REPLICATE_TOKEN").unwrap()),
        )
        .header("Content-Type", "application/json")
        .body(
            json!({
                "input": { "prompt": prompt },
                "stream": true
            })
            .to_string(),
        )
        .send()
        .await
    {
        Ok(r) => match r.json::<ReplicateSubmit>().await {
            Ok(j) => j,
            Err(e) => {
                return Err(ChatError {
                    message: format!("Failed to parse submit response: {:#?}", e),
                })
            }
        },
        Err(e) => {
            return Err(ChatError {
                message: format!("Failed to submit request: {:#?}", e),
            })
        }
    };

    let stream_url = &submit_response.urls.stream;
    let prediction_id = &submit_response.id;

    let mut es =
        EventSource::new(reqwest_client.get(stream_url)).expect("Failed to create event source");

    let mut full_output = String::new();
    let mut last_update = Instant::now();
    let update_interval = Duration::from_millis(750); // Update Discord every 750ms to avoid rate limits

    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Message(message)) => match message.event.as_str() {
                "output" => {
                    full_output.push_str(&message.data);

                    if last_update.elapsed() >= update_interval {
                        let mut truncated_output = full_output.clone();
                        if truncated_output.len() >= 4096 {
                            truncated_output.truncate(4092);
                            truncated_output += "...";
                        }

                        interaction_client
                            .update_response(interaction_token)
                            .embeds(Some(&[
                                prompt_embed(prompt, prediction_id),
                                EmbedBuilder::new()
                                    .title("Processing...")
                                    .color(0x5e35b1)
                                    .description(truncated_output)
                                    .footer(EmbedFooterBuilder::new(prediction_id))
                                    .build(),
                            ]))
                            .await
                            .ok();
                        last_update = Instant::now();
                    }
                }
                "done" => {
                    es.close();
                    break;
                }
                "error" => {
                    es.close();
                    return Err(ChatError {
                        message: format!("An error occurred in the stream: {}", message.data),
                    });
                }
                _ => {}
            },
            Err(e) => {
                es.close();
                if full_output.is_empty() {
                    return Err(ChatError {
                        message: format!("Stream connection error: {:#?}", e),
                    });
                }
                break;
            }
            _ => {} // Ignore Open events
        }
    }

    if full_output.is_empty() {
        interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[
                prompt_embed(prompt, prediction_id),
                EmbedBuilder::new()
                    .title("Error")
                    .color(0xe53935)
                    .description("The model finished but generated no output.")
                    .footer(EmbedFooterBuilder::new(prediction_id))
                    .build(),
            ]))
            .await
            .ok();
    } else {
        if full_output.len() >= 4096 {
            full_output.truncate(4092);
            full_output += "...";
        }
        interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[
                prompt_embed(prompt, prediction_id),
                EmbedBuilder::new()
                    .title("Succeeded")
                    .color(0x43a047)
                    .description(full_output)
                    .footer(EmbedFooterBuilder::new(prediction_id))
                    .build(),
            ]))
            .await
            .ok();
    }

    Ok(())
}
