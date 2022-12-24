use std::env;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{
    EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};

use super::{CommandHandler, CommandHandlerData};

#[derive(Debug, PartialEq)]
enum Status {
    Finished,
    Processing,
    Waiting,
}

#[derive(CommandOption, CreateOption)]
enum DiffusionModel {
    #[option(name = "Stable Diffusion 2.1", value = "stable_diffusion_2.1")]
    SD21,
    #[option(name = "Stable Diffusion 1.5", value = "stable_diffusion")]
    SD15,
    #[option(name = "Anything V3.0", value = "Anything Diffusion")]
    AnythingV3,
    #[option(name = "Dreamlike Diffusion", value = "Dreamlike Diffusion")]
    Dreamlike,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "horde", desc = "Create an image with the Stable Horde 👺")]
pub struct HordeCommand {
    /// Prompt for the model to generate
    prompt: String,
    /// Define pre-trained weights for the model
    model: Option<DiffusionModel>,
}

#[derive(Serialize, Deserialize)]
struct HordeParams<'a> {
    sampler_name: &'a str,
    steps: u8,
}

#[derive(Serialize, Deserialize)]
struct HordeSubmit<'a> {
    prompt: &'a str,
    params: HordeParams<'a>,
    nsfw: bool,
    censor_nsfw: bool,
    models: Vec<&'a str>,
}

#[derive(Deserialize)]
struct HordeResponse {
    id: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct HordePoll {
    done: bool,
    faulted: bool,
    wait_time: f32,
    queue_position: f32,
}

#[derive(Deserialize)]
struct HordeFinal {
    generations: Vec<HordeGeneration>,
}

#[derive(Deserialize)]
struct HordeGeneration {
    worker_name: String,
    img: String,
}

#[async_trait]
impl CommandHandler for HordeCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let interaction_client = command_handler_data.interaction_client;
        let reqwest_client = command_handler_data.reqwest_client;

        let (style, model_name, model_version) = match &self.model {
            Some(m) => match m {
                DiffusionModel::SD21 => {
                    ("", "Stable Diffusion [2.1]", DiffusionModel::SD21.value())
                }
                DiffusionModel::SD15 => {
                    ("", "Stable Diffusion [1.5]", DiffusionModel::SD15.value())
                }
                DiffusionModel::AnythingV3 => {
                    ("", "Anything V3.0", DiffusionModel::AnythingV3.value())
                }
                DiffusionModel::Dreamlike => (
                    "dreamlikeart",
                    "Dreamlike Diffusion",
                    DiffusionModel::Dreamlike.value(),
                ),
            },
            None => ("", "Stable Diffusion [1.5]", DiffusionModel::SD15.value()),
        };

        let mod_prompt = if !style.is_empty() {
            format!("{}, {}", &self.prompt, style)
        } else {
            self.prompt.to_string()
        };
        let prompt = mod_prompt.as_str();

        interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![embed_with_prompt_and_model(
                            "Submitting",
                            0xF4511E,
                            prompt,
                            model_name,
                        )
                        .build()]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();

        match horde(
            &reqwest_client,
            prompt,
            model_name,
            model_version,
            &interaction_client,
            interaction_token,
        )
        .await
        {
            Ok(_) => return,
            Err(e) => {
                interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[embed_with_prompt_and_model(
                        "Failed", 0xE53935, prompt, model_name,
                    )
                    .field(EmbedFieldBuilder::new(
                        "Error",
                        format!(
                            r#"An exception was caught to save the bot from crashing:
                            `{}`"#,
                            e.message
                        ),
                    ))
                    .build()]))
                    .unwrap()
                    .await
                    .ok();
            }
        }
    }
}

struct HordeError {
    message: String,
}

async fn horde(
    reqwest_client: &Client,
    prompt: &str,
    model_name: &str,
    model_version: &str,
    interaction_client: &InteractionClient<'_>,
    interaction_token: &str,
) -> Result<(), HordeError> {
    let submit_request = reqwest_client
        .post("https://stablehorde.net/api/v2/generate/async")
        .header("apikey", env::var("HORDE_TOKEN").unwrap())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            json!(&HordeSubmit {
                prompt,
                params: HordeParams {
                    sampler_name: "k_euler_a",
                    steps: 50,
                },
                nsfw: false,
                censor_nsfw: true,
                models: vec![model_version],
            })
            .to_string(),
        )
        .send()
        .await;

    let id = match submit_request {
        Ok(r) => match r.json::<HordeResponse>().await {
            Ok(j) => match j.id {
                Some(id) => id,
                None => {
                    return Err(HordeError {
                        message: format!("{:#?}", j.message.unwrap()),
                    })
                }
            },
            Err(e) => {
                return Err(HordeError {
                    message: format!("{:#?}", e),
                })
            }
        },
        Err(e) => {
            return Err(HordeError {
                message: format!("{:#?}", e),
            })
        }
    };

    interaction_client
        .update_response(interaction_token)
        .embeds(Some(&[embed_with_prompt_and_model(
            "Submitted",
            0x00897B,
            prompt,
            model_name,
        )
        .footer(EmbedFooterBuilder::new(&id))
        .build()]))
        .unwrap()
        .await
        .ok();

    let start = SystemTime::now();

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let since_start = SystemTime::now()
            .duration_since(start)
            .expect("Time went backwards");

        if since_start.as_secs() > 120 {
            return Err(HordeError {
                message: "The command timed out after 2 minutes".to_string(),
            });
        }

        let poll_request = reqwest_client
            .get(format!(
                "https://stablehorde.net/api/v2/generate/check/{}",
                &id
            ))
            .send()
            .await;

        let poll_response = match poll_request {
            Ok(r) => match r.json::<HordePoll>().await {
                Ok(j) => j,
                Err(e) => {
                    print!("{:#?}", e);
                    continue;
                }
            },
            Err(e) => {
                return Err(HordeError {
                    message: format!("{:#?}", e),
                })
            }
        };

        let status = if poll_response.queue_position > 0.0 {
            Status::Waiting
        } else if poll_response.done {
            Status::Finished
        } else if poll_response.faulted {
            return Err(HordeError {
                message: "An unrecoverable fault has occurred".to_string(),
            });
        } else {
            Status::Processing
        };

        if status != Status::Finished {
            interaction_client
                .update_response(interaction_token)
                .embeds(Some(&[embed_with_prompt_and_model(
                    "Pending", 0x5E35B1, prompt, model_name,
                )
                .field(EmbedFieldBuilder::new(
                    "Status",
                    format!(
                        r#"`{:#?}`
                        Wait time is {} seconds.
                        Queue position is {}."#,
                        status, poll_response.wait_time, poll_response.queue_position
                    ),
                ))
                .footer(EmbedFooterBuilder::new(&id))
                .build()]))
                .unwrap()
                .await
                .ok();
            continue;
        }

        let final_request = reqwest_client
            .get(format!(
                "https://stablehorde.net/api/v2/generate/status/{}",
                &id
            ))
            .send()
            .await;

        let final_response = match final_request {
            Ok(r) => match r.json::<HordeFinal>().await {
                Ok(j) => j,
                Err(e) => {
                    return Err(HordeError {
                        message: format!("{:#?}", e),
                    })
                }
            },
            Err(e) => {
                return Err(HordeError {
                    message: format!("{:#?}", e),
                })
            }
        };

        let generation = match final_response.generations.get(0) {
            Some(g) => g,
            None => {
                return Err(HordeError {
                    message: "The list of generated images was empty".to_string(),
                })
            }
        };

        let image = match base64::decode(generation.img.as_bytes()) {
            Ok(i) => i,
            Err(e) => {
                return Err(HordeError {
                    message: format!("{:#?}", e),
                })
            }
        };

        interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[embed_with_prompt_and_model(
                "Completed",
                0x43A047,
                prompt,
                model_name,
            )
            .field(EmbedFieldBuilder::new(
                "Info",
                format!(
                    "Your request was completed by worker `{}`",
                    generation.worker_name
                ),
            ))
            .image(ImageSource::attachment("image.webp").unwrap())
            .footer(EmbedFooterBuilder::new(&id))
            .build()]))
            .unwrap()
            .await
            .ok();

        interaction_client
            .update_response(interaction_token)
            .attachments(&[Attachment::from_bytes("image.webp".to_string(), image, 1)])
            .unwrap()
            .await
            .ok();

        return Ok(());
    }
}

fn embed_with_prompt_and_model(
    title: &str,
    color: u32,
    prompt: &str,
    model_name: &str,
) -> EmbedBuilder {
    EmbedBuilder::new()
        .title(title)
        .color(color)
        .field(EmbedFieldBuilder::new("Prompt", prompt))
        .field(EmbedFieldBuilder::new("Model", model_name))
}
