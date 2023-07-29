use std::env;

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
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};

use super::{CommandHandler, CommandHandlerData};

#[derive(CommandOption, CreateOption)]
enum StableStyle {
    #[option(name = "analog-film", value = "analog-film")]
    AnalogFilm,
    #[option(name = "anime", value = "anime")]
    Anime,
    #[option(name = "cinematic", value = "cinematic")]
    Cinematic,
    #[option(name = "comic-book", value = "comic-book")]
    ComicBook,
    #[option(name = "digital-art", value = "digital-art")]
    DigitalArt,
    #[option(name = "enhance", value = "enhance")]
    Enhance,
    #[option(name = "fantasy-art", value = "fantasy-art")]
    FantasyArt,
    #[option(name = "isometric", value = "isometric")]
    Isometric,
    #[option(name = "line-art", value = "line-art")]
    LineArt,
    #[option(name = "low-poly", value = "low-poly")]
    LowPoly,
    #[option(name = "modeling-compound", value = "modeling-compound")]
    ModelingCompound,
    #[option(name = "neon-punk", value = "neon-punk")]
    NeonPunk,
    #[option(name = "origami", value = "origami")]
    Origami,
    #[option(name = "photographic", value = "photographic")]
    Photographic,
    #[option(name = "pixel-art", value = "pixel-art")]
    PixelArt,
    #[option(name = "3d-model", value = "3d-model")]
    Model3D,
    #[option(name = "tile-texture", value = "tile-texture")]
    TileTexture,
}

#[derive(CommandOption, CreateOption)]
enum StableRatio {
    #[option(name = "square", value = "square")]
    Square,
    #[option(name = "portrait", value = "portrait")]
    Portrait,
    #[option(name = "landscape", value = "landscape")]
    Landscape,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "dream", desc = "Create an image with Stable Diffusion")]
pub struct DreamCommand {
    /// Prompt for the model to generate
    prompt: String,
    /// Define pre-trained weights for the model
    style: Option<StableStyle>,
    /// Select an aspect ratio for the final image
    ratio: Option<StableRatio>,
}

#[derive(Deserialize)]
struct StableImage {
    base64: String,
}

#[derive(Deserialize)]
struct StableResponse {
    message: Option<String>,
    artifacts: Option<Vec<StableImage>>,
}

struct AspectRatio {
    width: i16,
    height: i16,
}

impl ToString for AspectRatio {
    fn to_string(&self) -> String {
        return format!("{}x{}", self.width, self.height);
    }
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

        let prompt = &self.prompt;

        let style = self.style.as_ref().map(|f| f.value());

        let ratio = match self.ratio.as_ref() {
            Some(r) => match r {
                StableRatio::Square => AspectRatio {
                    width: 1024,
                    height: 1024,
                },
                StableRatio::Portrait => AspectRatio {
                    width: 768,
                    height: 1344,
                },
                StableRatio::Landscape => AspectRatio {
                    width: 1344,
                    height: 768,
                },
            },
            None => AspectRatio {
                width: 1024,
                height: 1024,
            },
        };

        interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Submitting")
                            .color(0x673AB7)
                            .field(EmbedFieldBuilder::new("Prompt", prompt))
                            .field(EmbedFieldBuilder::new(
                                "Style/Ratio",
                                format!("{}, {}", style.unwrap_or("None"), ratio.to_string()),
                            ))
                            .field(EmbedFieldBuilder::new("Ratio", ratio.to_string()))
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
            style,
            &ratio,
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
                        .field(EmbedFieldBuilder::new("Prompt", prompt))
                        .field(EmbedFieldBuilder::new(
                            "Style/Ratio",
                            format!("{}, {}", style.unwrap_or("None"), ratio.to_string()),
                        ))
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

#[derive(Serialize, Deserialize)]
struct StablePrompt<'a> {
    text: &'a str,
    weight: i16,
}

#[derive(Serialize, Deserialize)]
struct StableRequest<'a> {
    width: i16,
    height: i16,
    steps: i16,
    cfg_scale: i16,
    samples: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    style_preset: Option<&'a str>,
    text_prompts: Vec<StablePrompt<'a>>,
}

async fn dream(
    reqwest_client: &Client,
    prompt: &str,
    style: Option<&str>,
    ratio: &AspectRatio,
    interaction_client: &InteractionClient<'_>,
    interaction_token: &str,
) -> Result<(), DreamError> {
    let stable_request = StableRequest {
        width: ratio.width,
        height: ratio.height,
        steps: 40,
        cfg_scale: 7,
        samples: 1,
        style_preset: style,
        text_prompts: vec![StablePrompt {
            text: prompt,
            weight: 1,
        }],
    };

    let submit_request = reqwest_client
        .post("https://api.stability.ai/v1/generation/stable-diffusion-xl-1024-v1-0/text-to-image")
        .header(
            "Authorization",
            format!("Bearer {}", env::var("STABLE_KEY").unwrap()),
        )
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(json!(stable_request).to_string())
        .send()
        .await;

    let submit_response = match submit_request {
        Ok(r) => match r.json::<StableResponse>().await {
            Ok(j) => match j.artifacts {
                Some(v) => v,
                None => {
                    return Err(DreamError {
                        message: format!("{:#?}", j.message),
                    })
                }
            },
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

    let image_base64 = match submit_response.first() {
        Some(image) => &image.base64,
        None => todo!(),
    };

    let image = match base64::decode(image_base64) {
        Ok(i) => i,
        Err(e) => {
            return Err(DreamError {
                message: format!("{:#?}", e),
            })
        }
    };

    let filename = "image.png".to_string();

    interaction_client
        .update_response(interaction_token)
        .embeds(Some(&[EmbedBuilder::new()
            .title("Completed")
            .color(0x43A047)
            .field(EmbedFieldBuilder::new("Prompt", prompt))
            .field(EmbedFieldBuilder::new(
                "Style/Ratio",
                format!("{}, {}", style.unwrap_or("None"), ratio.to_string()),
            ))
            .image(ImageSource::attachment(&filename).unwrap())
            .build()]))
        .unwrap()
        .await
        .ok();

    interaction_client
        .update_response(interaction_token)
        .attachments(&[Attachment::from_bytes(filename, image, 1)])
        .unwrap()
        .await
        .ok();

    return Ok(());
}
