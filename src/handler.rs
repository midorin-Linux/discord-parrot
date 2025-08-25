use crate::Config;
use crate::voice::voicevox::client::Client as VoicevoxClient;
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
use sqlx::SqlitePool;
use tracing::{debug, info, warn, error, instrument};

pub struct Handler {
    voicevox_client: VoicevoxClient,
}

impl Handler {
    pub async fn new(config: Config) -> Result<Self> {
        debug!("Initializing handler...");

        let voicevox_client = VoicevoxClient::new(config.clone())?;
        
        debug!("Handler initialized");
        
        Ok(Self {
            voicevox_client,
        })
    }
}

#[async_trait]
impl EventHandler for Handler {
    #[instrument(skip(self, ctx, ready), fields(user_id = %ready.user.id, user_name = %ready.user.name))]
    async fn ready(&self, ctx: SerenityContext, ready: Ready) {
        info!("{} is connected to Discord!", ready.user.name);

        // let commands = GuildId::new(1233632516750184489)
        //     .set_commands(&ctx.http, vec![
        //         commands::join::register(),
        //     ]).await;
        // 
        // info!("Registered commands: {:?}", commands);
        info!("Ready!");
    }

    #[instrument(skip(self, _ctx, _resume))]
    async fn resume(&self, _ctx: SerenityContext, _resume: ResumedEvent) {
        info!("Resumed connection to Discord");
    }
}