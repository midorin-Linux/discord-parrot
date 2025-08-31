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

pub async fn run(ctx: &serenity::all::Context, interaction: &CommandInteraction, voicevox_client: &VoicevoxClient, voice_manager: &VoiceManager) -> Result<()> {
    interaction.defer(&ctx.http).await?;
    let (guild_id, voice_channel_id) = {
        let guild_id = match interaction.guild_id {
            Some(guild_id) => guild_id,
            None => {
                let response = CreateInteractionResponseFollowup::new().content("このコマンドはギルド内でのみ使えます").ephemeral(true);
                interaction.create_followup(ctx, response).await?;
                return Ok(());
            }
        };

        let voice_channel_id = {
            let user_id = interaction.user.id;
            let voice_channel_id = ctx.cache.guild(guild_id)
                .and_then(|guild| {
                    guild.voice_states.get(&user_id).and_then(|voice_state| voice_state.channel_id)
                });

            match voice_channel_id {
                Some(id) => id,
                None => {
                    let msg = if ctx.cache.guild(guild_id).is_none() {
                        "ギルド情報の取得に失敗しました"
                    } else {
                        "VCに参加している必要があります"
                    };
                    let response = CreateInteractionResponseFollowup::new().content(msg).ephemeral(true);
                    interaction.create_followup(ctx, response).await?;
                    return Ok(());
                }
            }
        };

        (guild_id, voice_channel_id)
    };
    let voice_channel_url = format!("https://discord.com/channels/{}/{}", guild_id.get(), voice_channel_id.get());

    match voice_manager.connect(ctx, guild_id, interaction.channel_id, voice_channel_id).await {
        Ok(_) => {
            let response_content = embed::simple_embed(&ctx, "接続しました", &format!("{} に接続しました！", voice_channel_url), 0x00ff00, ).await;

            let response = CreateInteractionResponseFollowup::new().embed(response_content);
            interaction.create_followup(ctx, response).await?;

            // 音声再生
            if let Err(e) =
                playback::play(&ctx, &voicevox_client, guild_id, "接続しました".to_string()).await
            {
                error!("Failed to play audio: {}", e);
            } else {
                debug!("Audio play request successfully");
            }

            Ok(())
        }
        Err(e) => {
            error!("Failed to connect to voice channel: {}", e);

            let response_content = embed::simple_embed(
                &ctx,
                "接続に失敗しました",
                &format!("VCへの接続に失敗しました:\n{}", e),
                0xff0000,
            )
                .await;

            let response = CreateInteractionResponseFollowup::new().embed(response_content);
            interaction.create_followup(ctx, response).await?;

            Err(anyhow::anyhow!("Failed to connect to voice channel: {}", e))
        }
    }
}

pub fn register() -> CreateCommand{
    CreateCommand::new("join").description("VCに参加し、読み上げ機能を有効化します")
}