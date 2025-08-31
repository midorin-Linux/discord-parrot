use crate::config::Config;
use crate::voice::voicevox::client::Client as VoicevoxClient;
use anyhow::Result;
use serenity::{
    all::{Context, GuildId},
    async_trait
};
use songbird::{
    events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent},
    input,
};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn, error, instrument};

struct DeleteFileOnEnd {
    path: PathBuf,
}

#[async_trait]
impl VoiceEventHandler for DeleteFileOnEnd {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::Track(_) => {
                if let Err(e) = fs::remove_file(&self.path).await {
                    warn!("Failed to deleted temp file: {} ({e})", self.path.display());
                } else {
                    debug!("Deleted temp file: {}", self.path.display());
                }
            }
            _ => {}
        }
        None
    }
}

pub async fn play(ctx: &Context, voicevox_client: &VoicevoxClient, guild_id: GuildId, text: String) -> Result<()> {
    let manager = songbird::get(ctx).await
        .ok_or_else(|| anyhow::anyhow!("Songbirdマネージャーの取得に失敗しました"))?;
    let call = manager.get(guild_id)
        .ok_or_else(|| anyhow::anyhow!("ボイスチャンネルに接続されていません"))?;

    let audio_query = voicevox_client
        .create_audio_query(&text, 8, 1.1)
        .await
        .map_err(|e| anyhow::anyhow!("音声クエリの生成に失敗しました: {}", e))?;

    let wav_data = voicevox_client
        .synthesis(&audio_query, 8)
        .await
        .map_err(|e| anyhow::anyhow!("音声合成に失敗しました: {}", e))?;

    let path = voicevox_client
        .create_wav_file(wav_data)
        .await
        .map_err(|e| anyhow::anyhow!("WAVファイルの作成に失敗しました: {}", e))?;

    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        Err(anyhow::anyhow!("音声ファイルが見つかりません: {}", path))?;
    }

    let source = input::File::new(path_buf.clone()).into();
    let handler = &mut *call.lock().await;
    let handle = handler.enqueue(source).await;

    let _ = handle.add_event(
        Event::Track(TrackEvent::End),
        DeleteFileOnEnd { path: path_buf.clone() },
    );
    let _ = handle.add_event(
        Event::Track(TrackEvent::Error),
        DeleteFileOnEnd { path: path_buf },
    );

    Ok(())
}

pub async fn skip_current_voice(ctx: &Context, guild_id: GuildId) -> Result<()> {
    let manager = songbird::get(ctx).await
        .ok_or_else(|| anyhow::anyhow!("Songbirdマネージャーの取得に失敗しました"))?;
    let call = manager.get(guild_id)
        .ok_or_else(|| anyhow::anyhow!("ボイスチャンネルに接続されていません"))?;

    let handler = &mut *call.lock().await;
    handler.queue().skip()?;
    Ok(())
}