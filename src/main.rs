mod handler;
mod config;
mod error;
mod voice;
mod commands;
mod embed;
mod cache;

use crate::config::Config;
use crate::handler::Handler;
use crate::voice::voicevox::client::Client as VoicevoxClient;

use anyhow::{Context, Result};
use serenity::{
    Client,
};
use songbird::SerenityInit;
use std::{
    env,
    path::Path,
};
use serenity::all::GatewayIntents;
use sqlx::SqlitePool;
use tracing::{debug, info, warn, error};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = EnvFilter::from_default_env()
        .add_directive("discord_bot_new=debug".parse()?)
        .add_directive(log_level.parse().context("Invalid log level")?);

    fmt::Subscriber::builder()
        .with_env_filter(filter)
        .init();

    let config = match Config::from_env() {
        Ok(config) => {
            info!("Loaded config from environment");
            config
        }
        Err(e) => {
            error!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = config.validate() {
        error!("Failed to validate config: {}", e);
        std::process::exit(1);
    }

    info!("-----Configuration-----");
    info!("Database URL: {}", config.database_url);
    info!("Discord Token: {}", if config.discord_token.is_empty() { "(empty)" } else { "(set)" });
    info!("Guild ID: {}", config.guild_id);
    info!("Voicevox URL: {}", config.voicevox_url);
    info!("Default Speaker ID: {}", config.default_speaker_id);
    info!("Default Speed Scale: {}", config.default_speed_scale);
    info!("Request Timeout (secs): {}", config.request_timeout_secs);
    info!("-----------------------");

    info!("Starting bot...");

    let intents = GatewayIntents::all();
    debug!("Set intents");

    let handler = Handler::new(config.clone()).await?;
    debug!("Created handler");

    debug!("Creating serenity client...");
    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(handler)
        .register_songbird()
        .await
        .context("Failed to create client")?;
    info!("Created serenity client");

    tokio::select! {
        res = client.start() => {
            info!("Discord client stopped: {:?}", res);
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C; shutting down");
        }
    }

    Ok(())
}