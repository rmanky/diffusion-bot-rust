use std::io::Cursor;
use std::{env, fmt};

use async_trait::async_trait;
use base64::{engine::general_purpose, DecodeError, Engine as _};
use image::{DynamicImage, GenericImageView, ImageError, ImageFormat};
use log::{error, info};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use twilight_http::client::InteractionClient;
use twilight_http::error::Error as TwilightHttpError;
use twilight_http::response::DeserializeBodyError;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::Attachment as ChannelAttachment;
use twilight_model::http::attachment::Attachment as HttpAttachment;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::{InteractionMarker, MessageMarker};
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};
use twilight_validate::message::MessageValidationError;

use crate::activity::get_random_qoute;

use super::{CommandHandler, CommandHandlerData};

const GEMINI_API_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image-preview:generateContent";
const MAX_ERROR_LENGTH: usize = 1000;

struct NanoError {
    message: String,
}

struct NanoOutput {
    text: Option<String>,
    image: Option<Vec<u8>>,
}

#[derive(Debug)]
enum Error {
    Http(reqwest::Error),
    Image(ImageError),
    Base64(DecodeError),
    DiscordApi(TwilightHttpError),
    Validation(MessageValidationError),
    DeserializeBody(DeserializeBodyError),
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP request failed: {}", e),
            Self::Image(e) => write!(f, "Image processing failed: {}", e),
            Self::Base64(e) => write!(f, "Base64 decoding failed: {}", e),
            Self::DiscordApi(e) => write!(f, "Discord API error: {}", e),
            Self::Validation(e) => write!(f, "Discord message validation error: {}", e),
            Self::DeserializeBody(e) => write!(f, "Failed to process Discord response: {}", e),
            Self::Json(e) => write!(f, "JSON parsing failed: {}", e),
        }
    }
}

impl From<MessageValidationError> for Error {
    fn from(e: MessageValidationError) -> Self {
        Error::Validation(e)
    }
}

impl From<DeserializeBodyError> for Error {
    fn from(e: DeserializeBodyError) -> Self {
        Error::DeserializeBody(e)
    }
}

macro_rules! from_error {
    ($from:ty, $to:ident) => {
        impl From<$from> for Error {
            fn from(e: $from) -> Self {
                Error::$to(e)
            }
        }
    };
}

from_error!(reqwest::Error, Http);
from_error!(ImageError, Image);
from_error!(DecodeError, Base64);
from_error!(TwilightHttpError, DiscordApi);
from_error!(serde_json::Error, Json);

#[derive(CommandModel, CreateCommand)]
#[command(name = "nano", desc = "Create an image with Gemini 2.5 Flash (🍌)")]
pub struct NanoCommand {
    /// Prompt for the model to generate.
    prompt: String,
    /// Optional main image to use as input.
    main_image: Option<ChannelAttachment>,
    /// Optional secondary image to use as input.
    secondary_image: Option<ChannelAttachment>,
}

#[async_trait]
impl CommandHandler for NanoCommand {
    async fn handle_command(
        &self,
        handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let client = handler_data.interaction_client;
        let reqwest_client = handler_data.reqwest_client;
        info!("'nano' command received.");
        if let Err(e) = self
            .run_command(&client, reqwest_client, interaction_id, interaction_token)
            .await
        {
            error!("Error executing 'nano' command: {}", e);
            send_error_message(&client, interaction_token, None, &e.to_string()).await;
        }
    }
}

impl NanoCommand {
    async fn run_command(
        &self,
        client: &InteractionClient<'_>,
        reqwest_client: Client,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) -> Result<(), Error> {
        client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: None,
                },
            )
            .await?;
        info!("Interaction deferred.");

        let resized_main = download_and_resize(&reqwest_client, self.main_image.as_ref()).await?;
        let resized_secondary =
            download_and_resize(&reqwest_client, self.secondary_image.as_ref()).await?;

        let (prompt_embed, prompt_attachment) = build_prompt_display(
            &self.prompt,
            resized_main.as_ref(),
            resized_secondary.as_ref(),
        )?;

        let embeds = [prompt_embed.build()];
        if let Some(attachment) = prompt_attachment {
            let attachments = [attachment];
            client
                .update_response(interaction_token)
                .embeds(Some(&embeds))
                .attachments(&attachments)
                .await?;
        } else {
            client
                .update_response(interaction_token)
                .embeds(Some(&embeds))
                .await?;
        }
        info!("Initial prompt message sent.");

        let followup_id = create_generating_followup(&client, interaction_token).await?;
        info!(
            "Followup created with ID {}. Calling Gemini API...",
            followup_id
        );

        match nano(
            &reqwest_client,
            &self.prompt,
            resized_main.as_ref(),
            resized_secondary.as_ref(),
        )
        .await
        {
            Ok(output) => {
                info!("nano function returned Ok. Preparing final update for followup.");
                send_success_followup(&client, interaction_token, followup_id, output).await?;
                info!("Final update sent successfully.");
            }
            Err(e) => {
                error!("nano function returned an error: {}", e.message);
                send_error_message(&client, interaction_token, Some(followup_id), &e.message).await;
                info!("Final error update sent successfully.");
            }
        }

        Ok(())
    }
}

async fn download_and_resize(
    client: &Client,
    attachment: Option<&ChannelAttachment>,
) -> Result<Option<DynamicImage>, Error> {
    if let Some(att) = attachment {
        info!("Downloading and resizing image from {}", att.url);
        let bytes = client.get(&att.url).send().await?.bytes().await?;
        let image = image::load_from_memory(&bytes)?;
        return Ok(Some(image.thumbnail(1024, 1024)));
    }
    Ok(None)
}

fn concat_images_horizontally(img1: &DynamicImage, img2: &DynamicImage) -> DynamicImage {
    let (w1, h1) = img1.dimensions();
    let (w2, h2) = img2.dimensions();
    let new_height = h1.max(h2);
    let mut combined = image::RgbaImage::new(w1 + w2, new_height);
    image::imageops::overlay(&mut combined, img1, 0, 0);
    image::imageops::overlay(&mut combined, img2, w1 as i64, 0);
    DynamicImage::ImageRgba8(combined)
}

fn build_prompt_display(
    prompt: &str,
    main_img: Option<&DynamicImage>,
    sec_img: Option<&DynamicImage>,
) -> Result<(EmbedBuilder, Option<HttpAttachment>), ImageError> {
    let mut embed = EmbedBuilder::new()
        .title("Prompt Details")
        .color(0x5865f2)
        .field(EmbedFieldBuilder::new("Prompt", prompt));

    let attachment_data = match (main_img, sec_img) {
        (Some(main), Some(sec)) => {
            info!("Two images found, creating concatenated prompt image.");
            let combined = concat_images_horizontally(main, sec);
            let mut bytes = Cursor::new(Vec::new());
            combined.write_to(&mut bytes, ImageFormat::Png)?;
            Some(("prompt_combined.png", bytes.into_inner()))
        }
        (Some(main), None) => {
            info!("One image found, creating prompt image.");
            let mut bytes = Cursor::new(Vec::new());
            main.write_to(&mut bytes, ImageFormat::Png)?;
            Some(("prompt_main.png", bytes.into_inner()))
        }
        _ => None,
    };

    if let Some((filename, bytes)) = attachment_data {
        embed = embed.image(ImageSource::attachment(filename).unwrap());
        Ok((
            embed,
            Some(HttpAttachment::from_bytes(filename.to_string(), bytes, 1)),
        ))
    } else {
        Ok((embed, None))
    }
}

async fn create_generating_followup(
    client: &InteractionClient<'_>,
    token: &str,
) -> Result<Id<MessageMarker>, Error> {
    let embed = EmbedBuilder::new()
        .title("Generating...")
        .description(get_random_qoute())
        .color(0x673ab7)
        .build();
    let followup = client
        .create_followup(token)
        .embeds(&[embed])
        .await?
        .model()
        .await?;
    Ok(followup.id)
}

async fn send_success_followup(
    client: &InteractionClient<'_>,
    token: &str,
    id: Id<MessageMarker>,
    output: NanoOutput,
) -> Result<(), Error> {
    let mut embed_builder = EmbedBuilder::new().title("Completed").color(0x43a047);

    if let Some(text) = output.text {
        embed_builder = embed_builder.field(EmbedFieldBuilder::new("Response", text));
    }

    let filename = "image.png".to_string();
    if output.image.is_some() {
        embed_builder = embed_builder.image(ImageSource::attachment(&filename).unwrap());
    }

    let embeds = [embed_builder.build()];

    if let Some(image_bytes) = output.image {
        let attachment = HttpAttachment::from_bytes(filename, image_bytes, 1);
        let attachments = [attachment];
        client
            .update_followup(token, id)
            .embeds(Some(&embeds))
            .attachments(&attachments)
            .await?;
    } else {
        client
            .update_followup(token, id)
            .embeds(Some(&embeds))
            .await?;
    }

    Ok(())
}

async fn send_error_message(
    client: &InteractionClient<'_>,
    token: &str,
    followup_id: Option<Id<MessageMarker>>,
    message: &str,
) {
    let mut error_message = message.to_string();
    if error_message.len() > MAX_ERROR_LENGTH {
        error_message.truncate(MAX_ERROR_LENGTH);
        error_message.push_str("...");
    }

    let embed = EmbedBuilder::new()
        .title("Failed")
        .color(0xe53935)
        .field(EmbedFieldBuilder::new(
            "Error",
            format!("```\n{}\n```", error_message),
        ))
        .build();

    if let Some(id) = followup_id {
        if let Err(e) = client
            .update_followup(token, id)
            .embeds(Some(&[embed]))
            .await
        {
            error!("Failed to send error followup: {}", e);
        }
    } else {
        if let Err(e) = client.update_response(token).embeds(Some(&[embed])).await {
            error!("Failed to send error response: {}", e);
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    prompt_feedback: Option<PromptFeedback>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Content,
    finish_reason: Option<String>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Content {
    parts: Vec<Part>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Part {
    text: Option<String>,
    inline_data: Option<InlineData>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    data: String,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PromptFeedback {
    block_reason: Option<String>,
}

fn image_to_json_part(image: &DynamicImage) -> Result<serde_json::Value, ImageError> {
    let mut buf = Cursor::new(Vec::new());
    image.write_to(&mut buf, ImageFormat::Png)?;
    let data = general_purpose::STANDARD.encode(buf.into_inner());
    Ok(json!({ "inline_data": { "mime_type": "image/png", "data": data } }))
}

async fn nano(
    reqwest_client: &Client,
    prompt: &str,
    main_image: Option<&DynamicImage>,
    secondary_image: Option<&DynamicImage>,
) -> Result<NanoOutput, NanoError> {
    let mut parts: Vec<serde_json::Value> = Vec::new();

    if let Some(image) = secondary_image {
        parts.push(image_to_json_part(image).map_err(|e| NanoError {
            message: format!("Failed to encode secondary image: {}", e),
        })?);
    }
    if let Some(image) = main_image {
        parts.push(image_to_json_part(image).map_err(|e| NanoError {
            message: format!("Failed to encode main image: {}", e),
        })?);
    }
    parts.push(json!({ "text": prompt }));

    let request_body = json!({ "contents": [{ "parts": parts }] });

    let response = reqwest_client
        .post(GEMINI_API_URL)
        .header("x-goog-api-key", env::var("GEMINI_API_KEY").unwrap())
        .json(&request_body)
        .send()
        .await
        .map_err(|e| NanoError {
            message: format!("Request failed: {}", e),
        })?;

    let status_code = response.status();
    let text = response.text().await.unwrap_or_default();

    if status_code != StatusCode::OK {
        return Err(NanoError {
            message: format!("API Error {}:\n{}", status_code, text),
        });
    }

    let gemini_response: GeminiResponse = serde_json::from_str(&text).map_err(|e| NanoError {
        message: format!("JSON Parse Error: {}\nResponse: {}", e, text),
    })?;

    if let Some(reason) = gemini_response
        .prompt_feedback
        .and_then(|fb| fb.block_reason)
    {
        return Err(NanoError {
            message: format!("Blocked by safety filter: {}", reason),
        });
    }

    let candidate = gemini_response
        .candidates
        .unwrap_or_default()
        .into_iter()
        .find(|c| c.finish_reason.as_deref() == Some("STOP"))
        .ok_or_else(|| NanoError {
            message: "No valid candidates in response.".to_string(),
        })?;

    let text = candidate.content.parts.iter().find_map(|p| p.text.clone());
    let image = candidate
        .content
        .parts
        .iter()
        .find_map(|p| p.inline_data.as_ref())
        .map(|data| general_purpose::STANDARD.decode(&data.data))
        .transpose()
        .map_err(|e| NanoError {
            message: format!("Base64 Decode Error: {}", e),
        })?;

    if text.is_none() && image.is_none() {
        return Err(NanoError {
            message: "Response contained no usable data.".to_string(),
        });
    }

    Ok(NanoOutput { text, image })
}
