use anyhow::Result;
use sqlx::SqlitePool;
use tracing::{debug, info, warn, error, instrument};

pub struct VoiceManager {
    pub pool: SqlitePool,
}

impl VoiceManager {
    pub fn new(pool: SqlitePool) -> Result<Self> {
        Ok(Self { pool })
    }

    pub async fn connect(&self, ctx: &serenity::all::Context, guild_id: serenity::model::id::GuildId, message_channel_id: serenity::all::ChannelId, voice_channel_id: serenity::model::id::ChannelId) -> Result<()> {
        let manager = songbird::get(&ctx)
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to get songbird manager"))?;

        let _handler = manager.join(guild_id, voice_channel_id).await.map_err(|e| {
            error!("Failed to connect to voice channel: {}", e);
            anyhow::anyhow!("Failed to connect to voice channel: {}", e)
        })?;

        let voice_channel_url = format!("https://discord.com/channels/{}/{}", guild_id.get(), voice_channel_id.get());
        info!("Connected to voice channel {}", voice_channel_url);

        sqlx::query(
            "INSERT OR REPLACE INTO sub_channel (guild_id, voice_channel_id, message_channel_id) VALUES (?, ?, ?)",
        )
            .bind(guild_id.get() as i64)
            .bind(voice_channel_id.get() as i64)
            .bind(message_channel_id.get() as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to record subscribe channel in the database: {}", e);
                anyhow::anyhow!("Failed to record subscribe channel in the database")
            })?;

        info!("Recorded subscribe channel in the database");
        Ok(())
    }

    pub async fn disconnect(&self, ctx: serenity::all::Context, guild_id: serenity::model::id::GuildId, channel_id: serenity::all::ChannelId) -> Result<()> {
        let manager = songbird::get(&ctx)
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to get songbird manager"))?;

        let _handler = manager.remove(guild_id).await.map_err(|e| {
            error!("Failed to disconnect from voice channel: {}", e);
            anyhow::anyhow!("Failed to disconnect from voice channel: {}", e)
        })?;

        sqlx::query("DELETE FROM sub_channel WHERE guild_id = ? AND (voice_channel_id = ? OR message_channel_id = ?)")
            .bind(guild_id.get() as i64)
            .bind(channel_id.get() as i64)
            .bind(channel_id.get() as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to remove voice channel record from database: {}", e);
                anyhow::anyhow!("Failed to remove voice channel record from database")
            })?;

        info!("Remove voice channel record from database");
        Ok(())
    }
}
