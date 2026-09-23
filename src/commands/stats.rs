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
// Cumulative /stats baseline through this score post, including the day-one corrections.
const SNAPSHOT_ANCHOR_MESSAGE_ID: Id<MessageMarker> = Id::new(1551583881873068095);
const SNAPSHOT_DAYS: usize = 491;
const SNAPSHOT_PLAYERS: &[(&str, u32, usize)] = &[
    ("150725833957441536", 2084, 485),
    ("302973340371517441", 1998, 490),
    ("481280459058184204", 1953, 483),
    ("656347629524877312", 1950, 465),
];

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
    penalized_score: u32,
    average_score: f32,
    days_played: usize,
}

async fn get_all_messages(
    data: &CommandHandlerData<'_>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut all_messages = Vec::new();
    let mut last_message_id = SNAPSHOT_ANCHOR_MESSAGE_ID;
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
                all_messages.push(message.content);
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

        // Vec<day, HashMap<user_id, score>>
        let mut daily_results: Vec<HashMap<String, u32>> = Vec::new();
        let mut all_participants: HashSet<String> = SNAPSHOT_PLAYERS
            .iter()
            .map(|(user_id, _, _)| (*user_id).to_string())
            .collect();

        for message in &messages {
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

        let mut leaderboard: Vec<PlayerStats> = all_participants
            .into_iter()
            .map(|user_id| {
                let (snapshot_total, snapshot_days_played) = SNAPSHOT_PLAYERS
                    .iter()
                    .find(|(snapshot_user_id, _, _)| *snapshot_user_id == user_id.as_str())
                    .map(|(_, total, days_played)| (*total, *days_played))
                    .unwrap_or((0, 0));
                let mut total_score = snapshot_total;
                let mut days_played = snapshot_days_played;

                for day in &daily_results {
                    if let Some(score) = day.get(&user_id) {
                        total_score += *score;
                        days_played += 1;
                    }
                }

                let total_days = SNAPSHOT_DAYS + daily_results.len();
                let penalized_score = total_score
                    + (total_days - days_played) as u32 * DEFAULT_SCORE;

                let average_score = if days_played > 0 {
                    total_score as f32 / days_played as f32
                } else {
                    0.0
                };

                PlayerStats {
                    user_id,
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
