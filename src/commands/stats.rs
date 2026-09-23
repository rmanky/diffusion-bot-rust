use std::collections::HashMap;
use std::fs;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::Message;
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::{ChannelMarker, InteractionMarker, MessageMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::builder::embed::ImageSource;

use super::stats_grid::{self, ScoreDay};
use super::{CommandHandler, CommandHandlerData};
use crate::utils::embed;

const TARGET_CHANNEL_ID: Id<ChannelMarker> = Id::new(946818381955366972);
const TARGET_BOT_ID: Id<UserMarker> = Id::new(1211781489931452447);
const DEFAULT_SCORE: u32 = 7;
const HISTORY_PATH: &str = "data/wordle_history.json";
const IMAGE_FILENAME: &str = "wordle-score-grid.png";

static USER_PATTERN_RE: Lazy<Regex> = Lazy::new(|| {
    let user_mention_pattern = r"<@\d+>";
    let mut patterns: Vec<String> = ALIASES
        .iter()
        .map(|alias| regex::escape(alias.name))
        .collect();
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

struct PlayerStats {
    user_id: String,
    total: u32,
    average: f32,
    days_played: usize,
}

#[derive(Default)]
struct PlayerTotal {
    score: u32,
    days_played: usize,
}

fn load_cached_history() -> Result<(CachedHistory, Id<MessageMarker>)> {
    let contents = fs::read_to_string(HISTORY_PATH).context("reading cached Wordle history")?;
    let history: CachedHistory =
        serde_json::from_str(&contents).context("parsing cached Wordle history")?;

    if history.days.is_empty()
        || history.days.last().map(|day| day.message_id.as_str())
            != Some(history.anchor_message_id.as_str())
    {
        return Err(anyhow!("cached history does not end at its anchor message"));
    }
    let anchor_id = history
        .anchor_message_id
        .parse::<u64>()
        .context("parsing history anchor message ID")?;

    for correction in &history.corrections {
        if !history
            .days
            .iter()
            .any(|day| day.message_id == correction.message_id)
        {
            return Err(anyhow!(
                "cached correction references missing message {}",
                correction.message_id
            ));
        }
    }

    Ok((history, Id::new(anchor_id)))
}

async fn get_all_messages(
    data: &CommandHandlerData<'_>,
    anchor_message_id: Id<MessageMarker>,
) -> Result<Vec<Message>> {
    let mut score_messages = Vec::new();
    let mut cursor = anchor_message_id;
    let mut crawled = 0;

    loop {
        let response = data
            .twilight_client
            .channel_messages(TARGET_CHANNEL_ID)
            .after(cursor)
            .limit(100)
            .await?
            .model()
            .await?;

        let mut page: Vec<Message> = response;

        if page.is_empty() {
            break;
        }

        page.sort_by_key(|message| message.id.get());
        crawled += page.len();
        cursor = page.last().expect("page is non-empty").id;
        score_messages.extend(page.into_iter().filter(is_score_message));
    }

    log::info!(
        "Fetched {} score messages after the cached snapshot.",
        score_messages.len()
    );
    log::info!("Crawled {crawled} messages.");
    Ok(score_messages)
}

fn is_score_message(message: &Message) -> bool {
    message.author.id == TARGET_BOT_ID && message.content.contains("Your group is on")
}

fn user_id_from_token(token: &str) -> Option<String> {
    if let Some(user_id) = token
        .strip_prefix("<@")
        .and_then(|token| token.strip_suffix('>'))
    {
        return Some(user_id.to_owned());
    }

    ALIASES
        .iter()
        .find(|alias| alias.name == token)
        .map(|alias| alias.id.to_string())
}

fn parse_message_scores(content: &str) -> HashMap<String, u32> {
    let mut scores = HashMap::new();

    for line in content.lines().skip(1) {
        let cleaned_line = line.replace('\\', "");
        let Some((score_part, users_part)) = cleaned_line.split_once(':') else {
            continue;
        };
        let score = score_part
            .chars()
            .find(|character| character.is_ascii_digit() || *character == 'X')
            .and_then(|character| character.to_digit(10))
            .unwrap_or(DEFAULT_SCORE);

        for token in USER_PATTERN_RE
            .find_iter(users_part)
            .map(|matched| matched.as_str())
        {
            if let Some(user_id) = user_id_from_token(token) {
                scores.insert(user_id, score);
            }
        }
    }

    scores
}

fn assemble_score_days(history: CachedHistory, messages: &[Message]) -> Vec<ScoreDay> {
    let mut corrections: HashMap<String, HashMap<String, u32>> = history
        .corrections
        .into_iter()
        .map(|correction| (correction.message_id, correction.scores_added))
        .collect();
    let mut days = Vec::with_capacity(history.days.len() + messages.len());

    for day in history.days {
        let mut scores = day.scores;
        if let Some(added_scores) = corrections.remove(&day.message_id) {
            scores.extend(added_scores);
        }
        days.push(ScoreDay {
            timestamp: day.timestamp,
            scores,
        });
    }

    days.extend(messages.iter().map(|message| ScoreDay {
        timestamp: message.timestamp.iso_8601().to_string(),
        scores: parse_message_scores(&message.content),
    }));
    days
}

fn calculate_leaderboard(days: &[ScoreDay]) -> Vec<PlayerStats> {
    let total_days = days.len();
    let mut totals: HashMap<String, PlayerTotal> = HashMap::new();

    for day in days {
        for (user_id, score) in &day.scores {
            let total = totals.entry(user_id.clone()).or_default();
            total.score += *score;
            total.days_played += 1;
        }
    }

    let mut leaderboard: Vec<PlayerStats> = totals
        .into_iter()
        .map(|(user_id, total)| PlayerStats {
            user_id,
            total: total.score + (total_days - total.days_played) as u32 * DEFAULT_SCORE,
            average: total.score as f32 / total.days_played as f32,
            days_played: total.days_played,
        })
        .collect();
    leaderboard.sort_by(|a, b| a.average.total_cmp(&b.average));
    leaderboard
}

fn leaderboard_description(leaderboard: &[PlayerStats]) -> String {
    leaderboard
        .iter()
        .enumerate()
        .map(|(rank, player)| {
            format!(
                "**{}.** <@{}> Avg: **{:.2}** (Total: {}, Days: {})\n",
                rank + 1,
                player.user_id,
                player.average,
                player.total,
                player.days_played
            )
        })
        .collect()
}

async fn build_stats(data: &CommandHandlerData<'_>) -> Result<(String, Vec<u8>)> {
    let (history, anchor_id) = load_cached_history()?;
    let messages = get_all_messages(data, anchor_id).await?;
    let days = assemble_score_days(history, &messages);
    let description = leaderboard_description(&calculate_leaderboard(&days));
    let png =
        stats_grid::render_png(&days).map_err(|error| anyhow!("rendering stats image: {error}"))?;
    Ok((description, png))
}

async fn respond_with_error(
    data: &CommandHandlerData<'_>,
    interaction_token: &str,
    error: anyhow::Error,
) {
    let error_message = format!("{error:#}");
    log::error!("{error_message}");
    let error_embed = embed::failure(&error_message).build();
    data.interaction_client
        .update_response(interaction_token)
        .embeds(Some(&[error_embed]))
        .await
        .ok();
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

        let (description, png) = match build_stats(&command_handler_data).await {
            Ok(stats) => stats,
            Err(error) => {
                respond_with_error(&command_handler_data, interaction_token, error).await;
                return;
            }
        };

        let attachment = Attachment::from_bytes(IMAGE_FILENAME.to_owned(), png, 1);
        let final_embed = embed::success()
            .title("Wordle Leaderboard")
            .description(&description)
            .image(ImageSource::attachment(IMAGE_FILENAME).unwrap())
            .build();

        command_handler_data
            .interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[final_embed]))
            .attachments(&[attachment])
            .await
            .ok();
    }
}
