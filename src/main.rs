use dotenv::dotenv;
use poise::{
    serenity_prelude::{self as serenity, CacheHttp, CreateEmbed},
    CreateReply, ReplyHandle,
};
use reqwest::{header::AUTHORIZATION, Response};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
// User data, which is stored and accessible in all command invocations
#[derive(Debug, Clone)]
struct Data {
    client: reqwest::Client,
    replicate_token: String,
    stable_version: String,
}

async fn unified_error(ctx: Context<'_>, thread: &ReplyHandle<'_>, description: &String) {
    let mut embed = CreateEmbed::default();
    embed.title("❌ An Error Occurred").description(description);

    thread
        .edit(ctx, |f| {
            *f = CreateReply {
                content: Some("".to_string()),
                embeds: vec![embed],
                ..Default::default()
            };
            f
        })
        .await
        .unwrap();
}

async fn dream_request(data: &Data, prompt: &String) -> Result<Response, reqwest::Error> {
    let client = &data.client;
    let stable_version = &data.stable_version;
    let replicate_token = &data.replicate_token;
    return client
        .post("https://api.replicate.com/v1/predictions")
        .header(AUTHORIZATION, format!("Token {replicate_token}"))
        .json(&serde_json::json!({
            "version": stable_version,
            "input": {
                "prompt": prompt
            }
        }))
        .send()
        .await;
}

async fn dream_check(data: &Data, id: &String) -> Result<Response, reqwest::Error> {
    let client = &data.client;
    let replicate_token = &data.replicate_token;
    return client
        .get(format!("https://api.replicate.com/v1/predictions/{id}"))
        .header(AUTHORIZATION, format!("Token {replicate_token}"))
        .send()
        .await;
}

#[poise::command(slash_command)]
async fn dream(ctx: Context<'_>, prompt: String) -> Result<(), Error> {
    ctx.defer().await?;

    let thread = ctx
        .send(|m| {
            m.embed(|e| {
                e.title("⌚ Request Received")
                    .field("Prompt", &prompt, false)
            })
        })
        .await?;

    let data = Arc::new(ctx.data().clone());
    let res = dream_handler(data, &prompt).await;

    match res {
        Ok(image) => {
            let mut embed = CreateEmbed::default();
            embed
                .title("✅ Request Completed")
                .image(image)
                .field("Prompt", &prompt, false);

            thread
                .edit(ctx, |f| {
                    *f = CreateReply {
                        content: Some("".to_string()),
                        embeds: vec![embed],
                        ..Default::default()
                    };
                    f
                })
                .await
                .unwrap();
        }
        Err(err) => unified_error(ctx, &thread, &err.to_string()).await,
    }
    Ok(())
}

/// Displays your or another user's account creation date
async fn dream_handler(data: Arc<Data>, prompt: &String) -> Result<String, Error> {
    let response = dream_request(&data, prompt).await?;

    let result = response.json::<serde_json::Value>().await?;
    let id = result["id"]
        .as_str()
        .ok_or("Response object did not contain an id")?
        .to_string();

    let poll = tokio::spawn({
        async move {
            loop {
                let status = dream_check(&data, &id).await.unwrap();
                let result = status.json::<serde_json::Value>().await.unwrap();
                let status = result["status"].as_str().unwrap();

                if status == "succeeded" {
                    return result["output"][0].as_str().unwrap().to_string();
                }

                sleep(Duration::from_secs(2)).await;
            }
        }
    });

    return poll
        .await
        .map_err(|_| Err::<_, String>("Error...".into()).unwrap());
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Use `Framework::builder()` to create a framework builder and supply basic data to the framework:
    poise::Framework::builder()
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN in .env"))
        .intents(serenity::GatewayIntents::non_privileged())
        .user_data_setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                // construct user data here (invoked when bot connects to Discord)
                let commands = &framework.options().commands;
                let create_commands = poise::builtins::create_application_commands(&commands);

                serenity::Command::set_global_application_commands(ctx.http(), |b| {
                    *b = create_commands;
                    b
                })
                .await?;

                Ok(Data {
                    client: reqwest::Client::new(),
                    replicate_token: std::env::var("REPLICATE_TOKEN")
                        .expect("missing REPLICATE_TOKEN in .env"),
                    stable_version: std::env::var("STABLE_VERSION")
                        .expect("missing STABLE_VERSION in .env"),
                })
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![dream()],
            ..Default::default()
        })
        .run()
        .await
        .unwrap();
}
