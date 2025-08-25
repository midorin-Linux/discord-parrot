use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde_json::{Value, json};
use tracing::{debug, info, warn, error, instrument};
use url::Url;

pub struct Audio {
    voicevox_client: HttpClient,
    voicevox_url: Url,
}

impl Audio {
    pub fn new(voicevox_client: HttpClient, voicevox_url: Url) -> Result<Self> {
        Ok(Self {
            voicevox_client,
            voicevox_url,
        })
    }

    #[instrument(skip(self, text, speaker_id, speed_scale), fields(text = %text, speaker_id = %speaker_id, speed_scale = %speed_scale))]
    pub async fn create_audio_query(&self, text: &str, speaker_id: u8, speed_scale: f64) -> Result<String> {
        debug!("Sending audio query create request to voicevox");

        let mut audio_query_url = self.voicevox_url.join("/audio_query").context("Failed to join voicevox url")?;

        audio_query_url.query_pairs_mut().append_pair("text", text).append_pair("speaker", speaker_id.to_string().as_str());

        match self.voicevox_client.post(audio_query_url).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    info!("Audio query create successfully");
                    let audio_query_raw = res.text().await?;
                    let mut v: Value = serde_json::from_str(&audio_query_raw)?;

                    v["speedScale"] = json!(speed_scale);
                    debug!("Modified audio query: {:#?}\n", v);
                    Ok(serde_json::to_string(&v)?)
                } else {
                    warn!("Audio query create failed with status code {}", res.status());
                    Err(anyhow::anyhow!
                    (
                        "Audio query create failed with status code {}",
                        res.status())
                    )
                }
            }
            Err(e) => {
                error!("Failed to create audio query:\n{}", e);
                Err(anyhow::anyhow!("Failed to create audio query:\n{}", e))
            }
        }
    }

    #[instrument(skip(self, audio_query, speaker), fields(speaker = %speaker))]
    pub async fn synthesis(&self, audio_query: &str, speaker: u8) -> Result<bytes::Bytes> {
        debug!("Sending synthesize request to voicevox");

        let mut synthesis_url = self.voicevox_url
            .join("/synthesis")
            .context("Failed to join URL")?;

        synthesis_url.query_pairs_mut()
            .append_pair("speaker", &speaker.to_string());

        match self.voicevox_client.post(synthesis_url).body(audio_query.to_string()).send().await {
            Ok(res) => {
                let status_ok = res.status().is_success();
                if status_ok {
                    let wav_data = res.bytes().await.context("Failed to read response body")?;
                    info!("Synthesis successfully");
                    Ok(wav_data)
                } else {
                    warn!("Synthesis failed with status code {}", res.status());
                    Err(anyhow::anyhow!(
                        "Synthesis failed with status code {}",
                        res.status()
                    ))
                }
            }
            Err(e) => {
                error!("Failed to synthesis:\n{}", e);
                Err(anyhow::anyhow!("Failed to synthesis:\n{}", e))
            }
        }
    }

    #[instrument(skip(self, wav_bytes))]
    pub async fn create_wav_file(&self, wav_bytes: bytes::Bytes) -> Result<String> {
        let filename = Self::filename_uuid().await;
        let path = format!("temp/{}.wav", filename);

        tokio::fs::write(&path, wav_bytes).await?;

        debug!("Wrote audio data to file: {}", path);

        tokio::fs::metadata(&path).await?;
        Ok(path)
    }

    async fn filename_uuid() -> String {
        let uuid = uuid::Uuid::new_v4();
        debug!("Generated UUID: {}", uuid);
        uuid.to_string()
    }
}