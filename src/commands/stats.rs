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

// --- Constants ---
const TARGET_CHANNEL_ID: Id<ChannelMarker> = Id::new(946818381955366972);
const TARGET_BOT_ID: Id<UserMarker> = Id::new(1211781489931452447);
const DEFAULT_SCORE: u8 = 7; // Score for not participating or failing

// --- Static Regex & Data ---
// Using once_cell::sync::Lazy to compile regexes once, efficiently.
static USER_ID_CAPTURE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<@(\d+)>").unwrap());
static USER_PATTERN_RE: Lazy<Regex> = Lazy::new(|| {
    let user_mention_pattern = r"<@\d+>";
    let mut patterns: Vec<String> = ALIASES.iter().map(|a| regex::escape(a.name)).collect();
    patterns.push(user_mention_pattern.to_string());
    Regex::new(&patterns.join("|")).unwrap()
});

// Use PascalCase for the struct name and &'static str for compile-time constants.
pub struct Alias {
    pub name: &'static str,
    pub id: u64,
}

// A static slice of known user aliases to normalize data.
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

// A struct to hold the final calculated stats for each player.
struct PlayerStats {
    user_id: String,
    total_score: u32,
    average_score: f32,
    days_played: usize,
}

/// Fetches all relevant messages from the target channel until the "1 day streak" message is found.
async fn get_all_messages(
    data: &CommandHandlerData<'_>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut all_messages = Vec::new();
    let mut last_message_id: Option<Id<MessageMarker>> = None;
    let mut num_messages_crawled = 0;

    'outer: loop {
        // CORRECTED: This `if/else` block correctly handles the builder pattern's type state changes.
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
                        let retry_after = body_json["retry_after"].as_f64().unwrap_or(1.5);
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

        let mut found_end = false;
        num_messages_crawled += messages.len();
        last_message_id = messages.last().map(|m| m.id);

        for message in messages {
            if message.author.id == TARGET_BOT_ID && message.content.contains("Your group is on") {
                if message.content.contains("Your group is on a 1 day streak") {
                    found_end = true;
                }
                all_messages.push(message.content);
                if found_end {
                    break 'outer;
                }
            }
        }
    }

    log::info!("Fetched {} relevant messages.", all_messages.len());
    log::info!("Crawled {} messages.", num_messages_crawled);
    Ok(all_messages)
}

/// Given a token matched by our combined regex, return the canonical User ID string.
fn get_user_id_from_token(token: &str) -> Option<String> {
    // Priority 1: Check for a direct <@ID> mention
    if let Some(caps) = USER_ID_CAPTURE_RE.captures(token) {
        if let Some(id_match) = caps.get(1) {
            return Some(id_match.as_str().to_string());
        }
    }
    // Priority 2: Check for a matching alias
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
        // 1. Defer the response immediately.
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

        // 2. Fetch all messages.
        let messages = match get_all_messages(&command_handler_data).await {
            Ok(messages) => messages,
            Err(e) => {
                log::error!("Error fetching messages: {}", e);
                let err_embed = embed::failure(&format!("Failed to fetch messages: {}", e)).build();
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
                .description("No score messages found.")
                .build();
            command_handler_data
                .interaction_client
                .update_response(interaction_token)
                .embeds(Some(&[empty_embed]))
                .await
                .ok();
            return;
        }

        // 3. Single Parsing Pass:
        let mut daily_results: Vec<HashMap<String, u8>> = Vec::new();
        let mut all_participants: HashSet<String> = HashSet::new();

        for message in messages.iter().rev() {
            let mut daily_scores: HashMap<String, u8> = HashMap::new();
            for line in message.split('\n').skip(1) {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() < 2 {
                    continue;
                }

                let score_part = parts[0];
                let users_part = parts[1];

                let score_char = score_part
                    .chars()
                    .find(|c| c.is_ascii_digit() || *c == 'X')
                    .unwrap_or('7');
                let score = if score_char == 'X' {
                    DEFAULT_SCORE
                } else {
                    (score_char.to_digit(10).unwrap_or(DEFAULT_SCORE as u32)) as u8
                };

                for mat in USER_PATTERN_RE.find_iter(users_part) {
                    if let Some(user_id) = get_user_id_from_token(mat.as_str()) {
                        daily_scores.insert(user_id.clone(), score);
                        all_participants.insert(user_id);
                    }
                }
            }
            daily_results.push(daily_scores);
        }

        // 3.5 Day One Override
        daily_results[0].extend(HashMap::from([
            ("302973340371517441".to_string(), 4), // Raúl 3.0
            ("150725833957441536".to_string(), 6), // rmanky
            ("481280459058184204".to_string(), 6), // troyotter
        ]));

        // 4. Score Assembly and Calculation (UPDATED LOGIC):
        let total_game_days = daily_results.len();
        let mut leaderboard: Vec<PlayerStats> = all_participants
            .into_iter()
            .map(|user_id| {
                let mut total_score: u32 = 0;
                let mut participation_days: usize = 0;

                // Iterate through each day to calculate this user's stats.
                for day in &daily_results {
                    if let Some(score) = day.get(&user_id) {
                        // User played this day.
                        total_score += *score as u32;
                        participation_days += 1;
                    } else {
                        // User did not play; add penalty score for the average calculation.
                        total_score += DEFAULT_SCORE as u32;
                    }
                }

                // Average is over all days to keep it fair.
                let average_score = if total_game_days > 0 {
                    total_score as f32 / total_game_days as f32
                } else {
                    0.0
                };

                PlayerStats {
                    user_id,
                    total_score,
                    average_score,
                    days_played: participation_days,
                }
            })
            .collect();

        // Sort by total score, ascending (lower is better).
        leaderboard.sort_by(|a, b| a.total_score.partial_cmp(&b.total_score).unwrap());

        // 5. Format the Leaderboard for Display.
        let description = leaderboard
            .iter()
            .enumerate()
            .map(|(i, stats)| {
                format!(
                    "**{}.** <@{}>: Total: **{}** (Avg: {:.2}, Days: {})\n",
                    i + 1,
                    stats.user_id,
                    stats.total_score,
                    stats.average_score,
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
