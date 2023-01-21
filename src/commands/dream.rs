use std::env;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{
    EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};

use super::{CommandHandler, CommandHandlerData};

#[derive(CommandOption, CreateOption)]
enum DiffusionModel {
    #[option(
        name = "Stable Diffusion 1.5",
        value = "27b93a2413e7f36cd83da926f3656280b2931564ff050bf9575f1fdf9bcd7478"
    )]
    SD15,
    #[option(
        name = "Anything V3.0",
        value = "f410ed4c6a0c3bf8b76747860b3a3c9e4c8b5a827a16eac9dd5ad9642edce9a2"
    )]
    AnythingV3,
    #[option(
        name = "Elden Ring Diffusion",
        value = "03963421baa88bf6de8c30b357bf68b3964a56a9160a40a0448cd1d39607d836"
    )]
    Elden,
    #[option(
        name = "OpenJourney",
        value = "9936c2001faa2194a261c01381f90e65261879985476014a0a37a334593a05eb"
    )]
    OpenJourney,
    #[option(
        name = "Arcane Diffusion",
        value = "4cbb3f91f9ba049151efb8922fdecc6703d419ea682b87ff94c5876addabfb19"
    )]
    ArcaneDiffusion,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "dream", desc = "Create an image with Stable Diffusion")]
pub struct DreamCommand {
    /// Prompt for the model to generate
    prompt: String,
    /// Define pre-trained weights for the model
    model: Option<DiffusionModel>,
}

#[derive(Deserialize)]
struct ReplicateSubmit {
    id: String,
}

#[derive(Deserialize)]
struct ReplicatePoll {
    logs: String,
    status: String,
    output: Option<Vec<String>>,
}

#[async_trait]
impl CommandHandler for DreamCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let interaction_client = command_handler_data.interaction_client;
        let reqwest_client = command_handler_data.reqwest_client;

        let (style, version) = match &self.model {
            Some(m) => match m {
                DiffusionModel::SD15 => ("", DiffusionModel::SD15.value()),
                DiffusionModel::AnythingV3 => ("", DiffusionModel::AnythingV3.value()),
                DiffusionModel::Elden => ("elden ring style", DiffusionModel::Elden.value()),
                DiffusionModel::OpenJourney => ("mdjrny-v4 style", DiffusionModel::SD15.value()),
                DiffusionModel::ArcaneDiffusion => {
                    ("arcane style", DiffusionModel::ArcaneDiffusion.value())
                }
            },
            None => ("", DiffusionModel::SD15.value()),
        };

        let mut prompt = self.prompt.to_owned();
        if !style.is_empty() {
            prompt = format!("{}, {}", prompt, style);
        }

        interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Submitting")
                            .color(0xF4511E)
                            .field(EmbedFieldBuilder::new("Prompt", &prompt))
                            .build()]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();

        match dream(
            &reqwest_client,
            prompt.as_str(),
            version,
            &interaction_client,
            interaction_token,
        )
        .await
        {
            Ok(_) => return,
            Err(e) => {
                interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[EmbedBuilder::new()
                        .title("Failed")
                        .color(0xE53935)
                        .field(EmbedFieldBuilder::new("Prompt", &prompt))
                        .field(EmbedFieldBuilder::new("Error", format!("`{}`", e.message)))
                        .build()]))
                    .unwrap()
                    .await
                    .ok();
            }
        }
    }
}

struct DreamError {
    message: String,
}

async fn dream(
    reqwest_client: &Client,
    prompt: &str,
    version: &str,
    interaction_client: &InteractionClient<'_>,
    interaction_token: &str,
) -> Result<(), DreamError> {
    let submit_request = reqwest_client
        .post("https://api.replicate.com/v1/predictions")
        .header(
            "Authorization",
            format!("Token {}", env::var("REPLICATE_TOKEN").unwrap()),
        )
        .body(
            json!({
                "version": version,
                "input": {
                    "prompt": prompt,
                    "width": 512,
                    "height": 512,
                    "num_inference_steps": 40
                }
            })
            .to_string(),
        )
        .send()
        .await;

    let submit_response = match submit_request {
        Ok(r) => match r.json::<ReplicateSubmit>().await {
            Ok(j) => j,
            Err(e) => {
                return Err(DreamError {
                    message: format!("{:#?}", e),
                })
            }
        },
        Err(e) => {
            return Err(DreamError {
                message: format!("{:#?}", e),
            })
        }
    };

    interaction_client
        .update_response(interaction_token)
        .embeds(Some(&[EmbedBuilder::new()
            .title("Submitted")
            .color(0x00897B)
            .field(EmbedFieldBuilder::new("Prompt", prompt))
            .footer(EmbedFooterBuilder::new(&submit_response.id))
            .build()]))
        .unwrap()
        .await
        .ok();

    let start = SystemTime::now();

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let since_start = SystemTime::now()
            .duration_since(start)
            .expect("Time went backwards");

        if since_start.as_secs() > 90 {
            return Err(DreamError {
                message: "The command timed out after 90 seconds".to_string(),
            });
        }

        let poll_request = reqwest_client
            .get(format!(
                "https://api.replicate.com/v1/predictions/{}",
                submit_response.id
            ))
            .header(
                "Authorization",
                format!("Token {}", env::var("REPLICATE_TOKEN").unwrap()),
            )
            .send()
            .await;

        let poll_response = match poll_request {
            Ok(r) => match r.json::<ReplicatePoll>().await {
                Ok(j) => j,
                Err(_) => continue,
            },
            Err(e) => {
                return Err(DreamError {
                    message: format!("{:#?}", e),
                })
            }
        };

        let last_log = match poll_response.logs.split('\n').last() {
            Some(log) => log,
            None => {
                return Err(DreamError {
                    message: "Failed to retrieve the last line from the logs".to_string(),
                })
            }
        };

        interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[EmbedBuilder::new()
                .title("Pending")
                .color(0x5E35B1)
                .field(EmbedFieldBuilder::new("Prompt", prompt))
                .field(EmbedFieldBuilder::new("Status", &poll_response.status))
                .field(EmbedFieldBuilder::new(
                    "Last Log",
                    format!("`{}`", last_log),
                ))
                .footer(EmbedFooterBuilder::new(&submit_response.id))
                .build()]))
            .unwrap()
            .await
            .ok();

        if poll_response.status == "succeeded" {
            let image = match &poll_response.output {
                Some(v) => match v.first() {
                    Some(u) => match ImageSource::url(u) {
                        Ok(i) => i,
                        Err(e) => {
                            return Err(DreamError {
                                message: format!("{:#?}", e),
                            })
                        }
                    },
                    None => {
                        return Err(DreamError {
                            message: "The image list was present but empty".to_string(),
                        })
                    }
                },
                None => {
                    return Err(DreamError {
                        message: "The image list was missing".to_string(),
                    })
                }
            };

            interaction_client
                .update_response(interaction_token)
                .embeds(Some(&[EmbedBuilder::new()
                    .title("Completed")
                    .color(0x43A047)
                    .field(EmbedFieldBuilder::new("Prompt", prompt))
                    .image(image)
                    .footer(EmbedFooterBuilder::new(&submit_response.id))
                    .build()]))
                .unwrap()
                .await
                .ok();
            return Ok(());
        } else if poll_response.status == "failed" {
            return Err(DreamError {
                message: last_log.to_string(),
            });
        }
    }
}
