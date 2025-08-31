use crate::embed;
use crate::voice::voicevox::client::Client as VoicevoxClient;
use crate::voice::manager::VoiceManager;
use crate::voice::playback;
use anyhow::Result;
use serenity::{
    builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseFollowup},
    model::application::CommandInteraction,
};
use sqlx::SqlitePool;
use tracing::{info, error, instrument, debug};

pub async fn run(ctx: &serenity::all::Context, interaction: &CommandInteraction, voice_manager: &VoiceManager) -> Result<()> {
    interaction.defer(&ctx.http).await?;
    let guild_id = match interaction.guild_id {
        Some(guild_id) => guild_id,
        None => {
            let response = CreateInteractionResponseFollowup::new().content("このコマンドはギルド内でのみ使えます").ephemeral(true);
            interaction.create_followup(ctx, response).await?;
            return Ok(());
        }
    };

    match voice_manager.disconnect(ctx, guild_id, interaction.channel_id).await {
        Ok(_) => {
            let response_content = embed::simple_embed(&ctx, "切断しました", "ご利用していただきありがとうございました", 0xff0000).await;

            let response = CreateInteractionResponseFollowup::new().embed(response_content);
            interaction.create_followup(ctx, response).await?;

            Ok(())
        }
        Err(e) => {
            error!("Failed to connect to voice channel: {}", e);

            let response_content = embed::simple_embed(
                &ctx,
                "切断に失敗しました",
                &format!("VCからの切断に失敗しました:\n{}", e),
                0xff0000,
            )
                .await;

            let response = CreateInteractionResponseFollowup::new().embed(response_content);
            interaction.create_followup(ctx, response).await?;

            Err(anyhow::anyhow!("Failed to disconnect from voice channel: {}", e))
        }
    }
}

pub fn register() -> CreateCommand{
    CreateCommand::new("leave").description("参加しているVCから切断します")
}