use crate::commands::CommandDelegateData;
use activity::get_random_activity;
use commands::CommandDelegate;
use dotenv::dotenv;
use std::{env, error::Error, sync::Arc, time::Duration};
use twilight_cache_inmemory::DefaultInMemoryCache;
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};
use twilight_http::Client as HttpClient;
use twilight_model::{
    gateway::{payload::outgoing::UpdatePresence, presence::Status},
    id::{marker::ApplicationMarker, Id},
};

mod activity;
mod commands;
pub mod embed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv().ok();
    env_logger::init();

    let token = env::var("DISCORD_TOKEN")?;

    let intents = Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES;

    let mut shard = Shard::new(ShardId::new(0, 1), token.clone(), intents);
    let sender = shard.sender();

    tokio::spawn(async move {
        loop {
            let activity = get_random_activity();
            let presence =
                UpdatePresence::new(vec![activity], false, None, Status::Online).unwrap();

            if let Err(e) = sender.command(&presence) {
                log::error!("Failed to send presence update: {}", e);
            }

            tokio::time::sleep(Duration::from_secs(1800)).await;
        }
    });

    // We pass the Arc'd http client to our command data.
    // Note: You may need to update CommandDelegateData to accept an Arc<HttpClient>.
    let command_data = Arc::new(CommandDelegateData {
        reqwest_client: reqwest::Client::new(),
        twilight_client: HttpClient::new(token.clone()),
    });

    let application_id = command_data
        .twilight_client
        .current_user_application()
        .await?
        .model()
        .await?
        .id;

    let interaction_client = command_data.twilight_client.interaction(application_id);

    interaction_client
        .set_global_commands(&command_data.command_definitions())
        .await?;

    let cache = DefaultInMemoryCache::builder()
        .message_cache_size(10)
        .build();

    // The main event loop.
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(event) => event,
            Err(_) => continue,
        };

        cache.update(&event);

        tokio::spawn(handle_event(
            event,
            application_id,
            Arc::clone(&command_data),
        ));
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    application_id: Id<ApplicationMarker>,
    command_data: Arc<CommandDelegateData>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Event::InteractionCreate(i) = event {
        command_data.handle_interaction(i.0, application_id).await;
    }

    Ok(())
}
