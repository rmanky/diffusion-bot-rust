use std::collections::{HashMap, HashSet};
use std::time::Duration;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use twilight_http::error::ErrorType;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::Message;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::{ChannelMarker, InteractionMarker, MessageMarker, UserMarker};
use twilight_model::id::Id;

use super::{CommandHandler, CommandHandlerData};
use crate::utils::embed;

const TARGET_CHANNEL_ID: Id<ChannelMarker> = Id::new(946818381955366972);
const TARGET_BOT_ID: Id<UserMarker> = Id::new(1211781489931452447);
const DEFAULT_SCORE: u32 = 7;

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
#[command(name = "stats", desc = "Compute the Wordle leaderboard")]
pub struct StatsCommand {}

struct PlayerStats {
    user_id: String,
    total_score: u32,
    penalized_score: u32,
    average_score: f32,
    days_played: usize,
}

async fn get_all_messages(
    data: &CommandHandlerData<'_>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut all_messages = Vec::new();
    let mut last_message_id: Option<Id<MessageMarker>> = None;
    let mut num_messages_crawled = 0;

    'outer: loop {
        let result = if let Some(message_id) = last_message_id {
            data.twilight_client
                .channel_messages(TARGET_CHANNEL_ID)
                .before(message_id)
                .limit(100)
                .await
        } else {
            data.twilight_client
                .channel_messages(TARGET_CHANNEL_ID)
                .limit(100)
                .await
        };

        let messages: Vec<Message> = match result {
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

        num_messages_crawled += messages.len();
        last_message_id = messages.last().map(|m| m.id);

        for message in messages {
            if message.author.id != TARGET_BOT_ID {
                continue;
            }
            if message.content.contains("Your group is on a 1 day streak") {
                all_messages.push(message.content);
                break 'outer;
            } else if message.content.contains("Your group is on") {
                all_messages.push(message.content);
            }
        }
    }

    log::info!("Fetched {} relevant messages.", all_messages.len());
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

        let messages = match get_all_messages(&command_handler_data).await {
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

        if messages.is_empty() {
            let empty_embed = embed::success()
                .description("Found no score messages!")
                .build();
            command_handler_data
                .interaction_client
                .update_response(interaction_token)
                .embeds(Some(&[empty_embed]))
                .await
                .ok();
            return;
        }

        // Vec<day, HashMap<user_id, score>>
        let mut daily_results: Vec<HashMap<String, u32>> = Vec::new();
        // HashSet<user_id>
        let mut all_participants: HashSet<String> = HashSet::new();

        for message in messages.iter().rev() {
            let mut daily_scores: HashMap<String, u32> = HashMap::new();
            for line in message.split('\n').skip(1) {
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
                        daily_scores.insert(user_id.clone(), score);
                        all_participants.insert(user_id);
                    }
                }
            }
            daily_results.push(daily_scores);
        }

        // Day one overrides
        daily_results[0].extend(HashMap::from([
            ("302973340371517441".to_string(), 4), // Raúl 3.0
            ("150725833957441536".to_string(), 6), // rmanky
            ("481280459058184204".to_string(), 6), // troyotter
        ]));

        let mut leaderboard: Vec<PlayerStats> = all_participants
            .into_iter()
            .map(|user_id| {
                let mut total_score: u32 = 0;
                let mut penalized_score: u32 = 0;
                let mut days_played: usize = 0;

                for day in &daily_results {
                    if let Some(score) = day.get(&user_id) {
                        total_score += *score;
                        penalized_score += *score;
                        days_played += 1;
                    } else {
                        // User didn't play, add penalty
                        penalized_score += DEFAULT_SCORE;
                    }
                }

                let average_score = if days_played > 0 {
                    total_score as f32 / days_played as f32
                } else {
                    0.0
                };

                PlayerStats {
                    user_id,
                    total_score,
                    penalized_score,
                    average_score,
                    days_played,
                }
            })
            .collect();

        // Sort by average score, ascending (lower is better).
        leaderboard.sort_by(|a, b| a.average_score.partial_cmp(&b.average_score).unwrap());

        let description = leaderboard
            .iter()
            .enumerate()
            .map(|(i, stats)| {
                format!(
                    "**{}.** <@{}> Avg: **{:.2}** (Total: {}, Days: {})\n",
                    i + 1,
                    stats.user_id,
                    stats.average_score,
                    stats.penalized_score,
                    stats.days_played
                )
            })
            .collect::<String>();

        let final_embed = embed::success()
            .title("Wordle Leaderboard")
            .description(&description)
            .build();

        command_handler_data
            .interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[final_embed]))
            .await
            .ok();
    }
}
