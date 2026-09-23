use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use twilight_http::error::ErrorType;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::Message;
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::{ChannelMarker, InteractionMarker, MessageMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::builder::embed::ImageSource;

use super::{CommandHandler, CommandHandlerData};
use super::stats_grid::{self, ScoreDay};
use crate::utils::embed;

const TARGET_CHANNEL_ID: Id<ChannelMarker> = Id::new(946818381955366972);
const TARGET_BOT_ID: Id<UserMarker> = Id::new(1211781489931452447);
const DEFAULT_SCORE: u32 = 7;
const HISTORY_PATH: &str = "data/wordle_history.json";

static USER_ID_CAPTURE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<@(\d+)>").unwrap());
static USER_PATTERN_RE: Lazy<Regex> = Lazy::new(|| {
    let user_mention_pattern = r"<@\d+>";
    let mut patterns: Vec<String> = ALIASES.iter().map(|a| regex::escape(a.name)).collect();
    patterns.push(user_mention_pattern.to_string());
    Regex::new(&patterns.join("|")).unwrap()
});

pub struct Alias {
    pub name: &'static str,
    pub id: u64,
}

const ALIASES: &[Alias] = &[
    Alias {
        name: "@rmanky",
        id: 150725833957441536,
    },
    Alias {
        name: "@Raúl 3.0",
        id: 302973340371517441,
    },
    Alias {
        name: "@troyotter",
        id: 481280459058184204,
    },
    Alias {
        name: "@aaron_",
        id: 656347629524877312,
    },
];

#[derive(CommandModel, CreateCommand)]
#[command(name = "stats", desc = "Show the Wordle score grid")]
pub struct StatsCommand {}

#[derive(Deserialize)]
struct CachedHistory {
    anchor_message_id: String,
    days: Vec<CachedScoreDay>,
    #[serde(default)]
    corrections: Vec<CachedCorrection>,
}

#[derive(Deserialize)]
struct CachedScoreDay {
    message_id: String,
    timestamp: String,
    scores: HashMap<String, u32>,
}

#[derive(Deserialize)]
struct CachedCorrection {
    message_id: String,
    scores_added: HashMap<String, u32>,
    #[serde(rename = "reason")]
    _reason: String,
}

fn load_cached_history() -> Result<CachedHistory, Box<dyn std::error::Error + Send + Sync>> {
    let contents = fs::read_to_string(HISTORY_PATH)?;
    let history: CachedHistory = serde_json::from_str(&contents)?;

    if history.days.is_empty()
        || history.days.last().map(|day| day.message_id.as_str())
            != Some(history.anchor_message_id.as_str())
    {
        return Err("cached history does not end at its anchor message".into());
    }

    history.anchor_message_id.parse::<u64>()?;

    for correction in &history.corrections {
        if !history
            .days
            .iter()
            .any(|day| day.message_id == correction.message_id)
        {
            return Err(format!(
                "cached correction references missing message {}",
                correction.message_id
            )
            .into());
        }
    }

    Ok(history)
}

async fn get_all_messages(
    data: &CommandHandlerData<'_>,
    anchor_message_id: Id<MessageMarker>,
) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
    let mut all_messages = Vec::new();
    let mut last_message_id = anchor_message_id;
    let mut num_messages_crawled = 0;

    loop {
        let result = data
            .twilight_client
            .channel_messages(TARGET_CHANNEL_ID)
            .after(last_message_id)
            .limit(100)
            .await;

        let mut messages: Vec<Message> = match result {
            Ok(response) => response.model().await?,
            Err(e) => {
                if let ErrorType::Response { body, status, .. } = e.kind() {
                    if *status == 429 {
                        let body_json: serde_json::Value = serde_json::from_slice(body)?;
                        let retry_after = body_json["retry_after"].as_f64().unwrap_or(0.5);
                        log::warn!("Rate limited, retrying after {}s", retry_after);
                        tokio::time::sleep(Duration::from_secs_f64(retry_after)).await;
                        continue;
                    }
                }
                return Err(e.into());
            }
        };

        if messages.is_empty() {
            break;
        }

        // Discord returns `after` pages oldest-first. Sort explicitly so the
        // cursor and score processing stay correct if response ordering changes.
        messages.sort_by_key(|message| message.id.get());
        num_messages_crawled += messages.len();
        last_message_id = messages.last().expect("non-empty page").id;

        for message in messages {
            if message.author.id == TARGET_BOT_ID && message.content.contains("Your group is on") {
                all_messages.push(message);
            }
        }
    }

    log::info!("Fetched {} score messages after the cached snapshot.", all_messages.len());
    log::info!("Crawled {} messages.", num_messages_crawled);
    Ok(all_messages)
}

fn get_user_id_from_token(token: &str) -> Option<String> {
    if let Some(id_match) = USER_ID_CAPTURE_RE.captures(token).and_then(|c| c.get(1)) {
        return Some(id_match.as_str().to_string());
    }

    if let Some(alias) = ALIASES.iter().find(|a| a.name == token) {
        return Some(alias.id.to_string());
    }

    log::warn!("Could not resolve token to user ID: '{}'", token);
    None
}

#[async_trait]
impl CommandHandler for StatsCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        command_handler_data
            .interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: None,
                },
            )
            .await
            .ok();

        let history = match load_cached_history() {
            Ok(history) => history,
            Err(e) => {
                let error_msg = format!("Failed to read stats history: {}", e);
                log::error!("{}", error_msg);
                let err_embed = embed::failure(&error_msg).build();
                command_handler_data
                    .interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[err_embed]))
                    .await
                    .ok();
                return;
            }
        };
        let anchor_message_id = Id::new(history.anchor_message_id.parse::<u64>().unwrap());

        let messages = match get_all_messages(&command_handler_data, anchor_message_id).await {
            Ok(messages) => messages,
            Err(e) => {
                let error_msg = format!("Failed to fetch messages: {}", e);
                log::error!("{}", error_msg);
                let err_embed = embed::failure(&error_msg).build();
                command_handler_data
                    .interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[err_embed]))
                    .await
                    .ok();
                return;
            }
        };

        let mut cached_days = history.days;
        for correction in history.corrections {
            if let Some(day) = cached_days
                .iter_mut()
                .find(|day| day.message_id == correction.message_id)
            {
                day.scores.extend(correction.scores_added);
            }
        }

        let mut days: Vec<ScoreDay> = cached_days
            .into_iter()
            .map(|day| ScoreDay {
                timestamp: day.timestamp,
                scores: day.scores,
            })
            .collect();

        for message in &messages {
            let mut daily_scores: HashMap<String, u32> = HashMap::new();
            for line in message.content.split('\n').skip(1) {
                let cleaned_line = line.replace("\\", "");
                let parts: Vec<&str> = cleaned_line.split(':').collect();
                if parts.len() < 2 {
                    continue;
                }

                let score_part = parts[0];
                let users_part = parts[1];

                let score = score_part
                    .chars()
                    .find(|c| c.is_ascii_digit() || *c == 'X')
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(DEFAULT_SCORE);

                for mat in USER_PATTERN_RE.find_iter(users_part) {
                    if let Some(user_id) = get_user_id_from_token(mat.as_str()) {
                        daily_scores.insert(user_id, score);
                    }
                }
            }
            days.push(ScoreDay {
                timestamp: message.timestamp.iso_8601().to_string(),
                scores: daily_scores,
            });
        }

        let png = match stats_grid::render_png(&days) {
            Ok(png) => png,
            Err(e) => {
                let error_msg = format!("Failed to render stats image: {}", e);
                log::error!("{}", error_msg);
                let err_embed = embed::failure(&error_msg).build();
                command_handler_data
                    .interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[err_embed]))
                    .await
                    .ok();
                return;
            }
        };

        let filename = "wordle-score-grid.png".to_string();
        let final_embed = embed::success()
            .title("Wordle Score Grid")
            .image(ImageSource::attachment(&filename).unwrap())
            .build();
        let attachment = Attachment::from_bytes(filename, png, 1);

        command_handler_data
            .interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[final_embed]))
            .attachments(&[attachment])
            .await
            .ok();
    }
}
