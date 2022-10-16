use dotenv::dotenv;
use poise::{
    serenity_prelude::{self as serenity, CacheHttp, CreateEmbed},
    CreateReply, ReplyHandle,
};
use reqwest::{header::AUTHORIZATION, Response};
use tokio::time::{sleep, Duration};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
// User data, which is stored and accessible in all command invocations
struct Data {
    client: reqwest::Client,
    replicate_token: String,
    stable_version: String,
}

async fn send_message(
    ctx: Context<'_>,
    thread: &ReplyHandle<'_>,
    title: String,
    message: String,
    image: Option<String>,
) {
    let mut embed = CreateEmbed::default();
    embed.title(title).description(message);

    if let Some(v) = image {
        embed.image(v);
    }

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

async fn dream_request(ctx: Context<'_>, prompt: String) -> Result<Response, reqwest::Error> {
    let client = &ctx.data().client;
    let stable_version = &ctx.data().stable_version;
    let replicate_token = &ctx.data().replicate_token;

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

async fn dream_check(
    client: &reqwest::Client,
    replicate_token: &String,
    id: &String,
) -> Result<Response, reqwest::Error> {
    return client
        .get(format!("https://api.replicate.com/v1/predictions/{id}"))
        .header(AUTHORIZATION, format!("Token {replicate_token}"))
        .send()
        .await;
}

async fn handle_response(
    ctx: Context<'_>,
    thread: &ReplyHandle<'_>,
    response: Response,
) -> Result<String, String> {
    let result = response.json::<serde_json::Value>().await;
    let parsed = result.unwrap();
    let id = parsed["id"].as_str();

    match id {
        Some(id_str) => {
            send_message(
                ctx,
                thread,
                "✅ Success".to_string(),
                format!("Request submitted with `id` {id_str}, polling for response"),
                None,
            )
            .await;
            return Ok(id_str.to_string());
        }
        None => {
            send_message(
                ctx,
                thread,
                "❌ An Error Occurred".to_string(),
                "We did not receive an `id` back from Replicate".to_string(),
                None,
            )
            .await;
            return Err("We did not receive an `id` back from Replicate".to_string());
        }
    }
}

/// Displays your or another user's account creation date
#[poise::command(slash_command)]
async fn dream(ctx: Context<'_>, prompt: String) -> Result<(), Error> {
    ctx.defer().await?;

    sleep(Duration::from_millis(3000)).await;
    let thread = &ctx.say(format!("Your prompt was {prompt}")).await?;

    let result = dream_request(ctx, prompt).await;

    match result {
        Ok(response) => {
            let id = handle_response(ctx, thread, response).await.unwrap();
            let replicate_token = ctx.data().replicate_token.to_string();
            println!("{}", replicate_token);
            println!("{}", id);

            let poll = tokio::spawn(async move {
                let client = reqwest::Client::new();
                loop {
                    let status = dream_check(&client, &replicate_token, &id).await.unwrap();
                    let result = status.json::<serde_json::Value>().await.unwrap();
                    let status = result["status"].as_str().unwrap();

                    if status == "succeeded" {
                        return result["output"][0].as_str().unwrap().to_string();
                    }

                    sleep(Duration::from_secs(2)).await;
                }
            });

            let image = poll.await.unwrap();

            send_message(
                ctx,
                thread,
                "⌚ Done Waiting".to_string(),
                "The request was completed!".to_string(),
                Some(image),
            )
            .await;
        }
        Err(_) => {
            send_message(
                ctx,
                thread,
                "❌ An Error Occurred".to_string(),
                "Failed to submit request to Replicate".to_string(),
                None,
            )
            .await;
            return Ok(());
        }
    }

    Ok(())
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
