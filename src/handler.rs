use crate::Config;
use crate::voice::manager::VoiceManager;
use crate::voice::voicevox::client::Client as VoicevoxClient;
use crate::voice::playback;
use crate::voice::voicevox::format;
use anyhow::{Context, Result};
use serenity::{
    all::Context as SerenityContext,
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::EventHandler,
    model::{
        channel::Message,
        event::ResumedEvent,
        gateway::Ready,
        guild::Member,
        id::{GuildId, UserId},
        user::User,
    },
    prelude::*,
};
use serenity::all::Interaction;
use std::path::Path;
use sqlx::SqlitePool;
use tracing::{debug, info, warn, error, instrument};

pub struct Handler {
    guild_id: u64,
    pool: SqlitePool,
    voice_manager: VoiceManager,
    voicevox_client: VoicevoxClient,
}

impl Handler {
    pub async fn new(config: Config) -> Result<Self> {
        debug!("Initializing handler...");

        let guild_id = config.guild_id.parse::<u64>().context("Invalid guild ID")?;

        let pool = SqlitePool::connect(&config.database_url).await.context("Failed to connect to database")?;

        sqlx::query("CREATE TABLE IF NOT EXISTS sub_channel (id INTEGER PRIMARY KEY, guild_id INTEGER, voice_channel_id INTEGER, message_channel_id INTEGER)")
            .execute(&pool)
            .await
            .context("Failed to create database schema")?;
        info!("Database schema created");

        let voice_manager = VoiceManager::new(pool.clone())?;

        let voicevox_client = VoicevoxClient::new(config.clone())?;
        
        debug!("Handler initialized");
        
        Ok(Self {
            guild_id,
            pool,
            voice_manager,
            voicevox_client,
        })
    }
}

#[async_trait]
impl EventHandler for Handler {
    #[instrument(skip(self, ctx, msg), fields(
        channel_id = %msg.channel_id,
        user_id = %msg.author.id,
        content = %msg.content
    ))]
    async fn message(&self, ctx: serenity::all::Context, msg: Message) {
        if msg.author.bot {
            debug!("Ignoring bot message");
            return;
        }

        let is_voice_channel = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM sub_channel WHERE guild_id = ? AND (voice_channel_id = ? OR message_channel_id = ?))"
        )
            .bind(msg.guild_id.map_or(0, |id| id.get() as i64))
            .bind(msg.channel_id.get() as i64)
            .bind(msg.channel_id.get() as i64)
            .fetch_one(&self.pool)
            .await;

        match is_voice_channel {
            Ok(true) => {
                if msg.content.as_str() == "!skip" {
                    info!("Received skip command in voice channel: {}", msg.content);
                    let guild_id = msg.guild_id.unwrap();
                    if let Err(e) = playback::skip_current_voice(&ctx, guild_id).await {
                        error!("Failed to skip audio: {}", e);
                    } else {
                        debug!("Audio skip request successfully");
                    }
                } else {
                    info!("Received voicevox request: {}", msg.content);
                    let guild_id = msg.guild_id.unwrap();
                    let formatted_text = format::format_voicevox_message(&ctx, &msg).await;

                    if let Err(e) = playback::play(&ctx, &self.voicevox_client, guild_id, formatted_text).await {
                        error!("Failed to play audio: {}", e);
                    } else {
                        debug!("Audio play request successfully");
                    }
                }
            }
            Ok(false) => {
                debug!("Message in non-voice channel");
            }
            Err(e) => {
                error!("Database query failed: {}", e);
            }

        }
    }

    #[instrument(skip(self, ctx, ready), fields(user_id = %ready.user.id, user_name = %ready.user.name))]
    async fn ready(&self, ctx: SerenityContext, ready: Ready) {
        info!("{} is connected to Discord!", ready.user.name);

        let commands = GuildId::new(self.guild_id)
            .set_commands(&ctx.http, vec![
                crate::commands::join::register(),
                crate::commands::leave::register(),
                crate::commands::dictionary::register(),
            ]).await;

        info!("Registered commands: {:?}", commands);
        init_app(&self.voicevox_client).await.unwrap();
        info!("Ready!");
    }

    #[instrument(skip(self, _ctx, _resume))]
    async fn resume(&self, _ctx: SerenityContext, _resume: ResumedEvent) {
        info!("Resumed connection to Discord");
    }

    #[instrument(skip(self, ctx, interaction))]
    async fn interaction_create(&self, ctx: serenity::all::Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let options = command.data.options();
            let options_len = options.len();

            info!("Received command: {:?}", command.data.name);
            debug!(user_id = %command.user.id, channel_id = %command.channel_id, options_len, "Processing command interaction");

            if let Err(why) = match command.data.name.as_str() {
                "join" => {
                    crate::commands::join::run(&ctx, &command, &self.voicevox_client, &self.voice_manager).await
                },
                "leave" => {
                    crate::commands::leave::run(&ctx, &command, &self.voice_manager).await
                },
                "dictionary" => {
                    crate::commands::dictionary::run(&ctx, &command, &self.voicevox_client).await
                }
                _ => {
                    warn!("Unknown command: {}", command.data.name);
                    let data = CreateInteractionResponseMessage::new().content("不明なコマンドです");
                    let builder = CreateInteractionResponse::Message(data);
                    command.create_response(&ctx.http, builder).await.map_err(|e| anyhow::anyhow!(e))
                },
            } {
                error!("Error during command execution: {:?}", why);
            }
        } else {
            debug!("Received non-command interaction; ignoring");
        }
    }
}

async fn init_app(voicevox_client: &VoicevoxClient) -> Result<()> {
    info!("Initializing application");
    let pool = SqlitePool::connect("sqlite://discord.db").await?;
    match sqlx::query("DELETE FROM sub_channel")
        .execute(&pool)
        .await
    {
        Ok(_) => {
            info!("Cleared sub_channel table in the database");
        },
        Err(e) => {
            error!("Failed to cleared sub_channel table in the database: {}", e);
        }
    }

    let mut entries = tokio::fs::read_dir(Path::new("temp")).await?;

    let mut files_to_delete = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if tokio::fs::metadata(&path).await?.is_file() {
            files_to_delete.push(path);
        }
    }

    let mut delete_tasks = Vec::new();
    for file_path in files_to_delete {
        let task = tokio::spawn(async move {
            println!("Deleting file: {:?}", file_path);
            tokio::fs::remove_file(file_path).await
        });
        delete_tasks.push(task);
    }

    for task in delete_tasks {
        task.await??;
    }

    match tokio::fs::read_to_string("user_dict.json").await {
        Ok(dict_data) => {
            voicevox_client.import_dict(dict_data.as_str()).await?;
            info!("Application initialized");
            Ok(())
        }
        Err(e) => {
            error!("Failed to read user_dict.json: {}", e);
            Err(anyhow::anyhow!("Failed to initialize application"))
        }
    }
}