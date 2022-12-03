use crate::commands::CommandDelegateData;
use commands::CommandDelegate;
use dotenv::dotenv;
use futures::stream::StreamExt;
use std::{env, error::Error, sync::Arc};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{
    cluster::{Cluster, ShardScheme},
    Event, Intents,
};
use twilight_http::Client as HttpClient;
use twilight_model::id::{marker::ApplicationMarker, Id};

mod commands;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN")?;

    // Start a single shard.
    let scheme = ShardScheme::Range {
        from: 0,
        to: 0,
        total: 1,
    };

    // Specify intents requesting events about things like new and updated
    // messages in a guild and direct messages.
    let intents = Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES;

    let (cluster, mut events) = Cluster::builder(token.clone(), intents)
        .shard_scheme(scheme)
        .build()
        .await?;

    let cluster = Arc::new(cluster);

    tokio::spawn(async move {
        cluster.up().await;
    });

    let command_data = Arc::new(CommandDelegateData {
        reqwest_client: reqwest::Client::new(),
        twilight_client: HttpClient::new(token),
    });

    let application_id = command_data
        .twilight_client
        .current_user_application()
        .exec()
        .await?
        .model()
        .await?
        .id;

    let interaction_client = command_data.twilight_client.interaction(application_id);

    interaction_client
        .set_global_commands(&command_data.command_definitions())
        .exec()
        .await?
        .models()
        .await?;

    // Since we only care about messages, make the cache only process messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // Startup an event loop to process each event in the event stream as they
    // come in.
    while let Some((_, event)) = events.next().await {
        // Update the cache.
        cache.update(&event);

        // Spawn a new task to handle the event
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
