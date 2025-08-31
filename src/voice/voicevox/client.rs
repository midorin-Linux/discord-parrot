use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde_json::{Value, json};
use tracing::{debug, info, warn, error, instrument};
use url::Url;

#[derive(Debug)]
pub enum WordType {
    ProperNoun,
    CommonNoun,
    Verb,
    Adjective,
    Suffix,
}

fn set_word_type(word_type: WordType) -> String {
    match word_type {
        WordType::ProperNoun => "PROPER_NOUN",
        WordType::CommonNoun => "COMMON_NOUN",
        WordType::Verb => "VERB",
        WordType::Adjective => "ADJECTIVE",
        WordType::Suffix => "SUFFIX",
    }.to_string()
}

pub struct Client {
    voicevox_client: HttpClient,
    voicevox_url: Url,
}

impl Client {
    pub fn new(config: Config) -> Result<Self> {
        debug!("Initializing voicevox handler...");

        let voicevox_client = HttpClient::builder()
            .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
            .build()
            .context("Failed to create voicevox client")?;

        let voicevox_url = config.voicevox_url.clone();

        debug!("Voicevox handler initialized");

        Ok(Self {
            voicevox_client,
            voicevox_url,
        })
    }

    // Audio functionality
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

    // Dictionary functionality
    #[instrument(skip(self, surface), fields(surface = %surface))]
    pub async fn find_uuid_by_surface(&self, surface: &str) -> Result<Option<String>> {
        debug!("Find uuid by surface");

        let dict_content = self.get_user_dict().await?;
        let dict_json: Value = serde_json::from_str(&dict_content)?;

        if let Some(dict_obj) = dict_json.as_object() {
            for (uuid, word_data) in dict_obj {
                if let Some(word_surface) = word_data.get("surface").and_then(|s| s.as_str()) {
                    if word_surface == surface {
                        return Ok(Some(uuid.clone()));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Surface not found"))
    }

    #[instrument(skip(self))]
    pub async fn get_user_dict(&self) -> Result<String> {
        debug!("Sending get user dict request to voicevox");

        let user_dict_url = self.voicevox_url
            .join("/user_dict")
            .context("Failed to join URL")?;

        match self.voicevox_client.get(user_dict_url).send().await {
            Ok(res) => {
                let status_ok = res.status().is_success();
                if status_ok {
                    let user_dict_raw = res.text().await?;
                    info!("User dict get successfully");
                    Ok(user_dict_raw)
                } else {
                    warn!("User dict get failed with status code {}", res.status());
                    Err(anyhow::anyhow!("User dict get failed with status code {}", res.status()))
                }
            }
            Err(e) => {
                error!("Failed to get user dict:\n{}", e);
                Err(anyhow::anyhow!("Failed to get user dict:\n{}", e))
            }
        }
    }

    #[instrument(skip(self, surface, pronunciation, accent_type, word_type), fields(surface = %surface, pronunciation = %pronunciation, accent_type = %accent_type, word_type = ?word_type))]
    pub async fn add_dict_word(&self, surface: &str, pronunciation: &str, accent_type: u8, word_type: Option<WordType>) -> Result<()> {
        debug!("Sending add word user dict word request to voicevox");

        let word_type_string = if let Some(word_type) = word_type {
            set_word_type(word_type)
        } else {
            set_word_type(WordType::ProperNoun)
        };

        let mut user_dict_word_url = self.voicevox_url
            .join("/user_dict_word")
            .context("Failed to join URL")?;

        user_dict_word_url.query_pairs_mut()
            .append_pair("surface", surface)
            .append_pair("pronunciation", pronunciation)
            .append_pair("accent_type", accent_type.to_string().as_str())
            .append_pair("word_type", &word_type_string)
            .append_pair("priority", "10");

        match self.voicevox_client.post(user_dict_word_url).send().await {
            Ok(res) => {
                let status_ok = res.status().is_success();
                if status_ok {
                    info!("User dict word add successfully");
                    Ok(())
                } else {
                    warn!("User dict word add failed with status code {}", res.status());
                    Err(anyhow::anyhow!("User dict word add failed with status code {}", res.status()))
                }
            }
            Err(e) => {
                error!("Failed to add user dict word:\n{}", e);
                Err(anyhow::anyhow!("Failed to add user dict word:\n{}", e))
            }
        }
    }

    pub async fn rewrite_dict_word(&self, surface: &str, pronunciation: &str, accent_type: u8, word_type: Option<WordType>) -> Result<()> {
        debug!("Sending rewrite word user dict word request to voicevox");

        let word_type_string = if let Some(word_type) = word_type {
            set_word_type(word_type)
        } else {
            set_word_type(WordType::ProperNoun)
        };

        let mut user_dict_word_url = self.voicevox_url
            .join("/user_dict_word")
            .context("Failed to join URL")?;

        user_dict_word_url.query_pairs_mut()
            .append_pair("surface", surface)
            .append_pair("pronunciation", pronunciation)
            .append_pair("accent_type", accent_type.to_string().as_str())
            .append_pair("word_type", &word_type_string)
            .append_pair("priority", "10");

        match self.voicevox_client.put(user_dict_word_url).send().await {
            Ok(res) => {
                let status_ok = res.status().is_success();
                if status_ok {
                    info!("User dict word rewrite successfully");
                    Ok(())
                } else {
                    warn!("User dict word rewrite failed with status code {}", res.status());
                    Err(anyhow::anyhow!("User dict word rewrite failed with status code {}", res.status()))
                }
            }
            Err(e) => {
                error!("Failed to rewrite user dict word:\n{}", e);
                Err(anyhow::anyhow!("Failed to rewrite user dict word:\n{}", e))
            }
        }
    }

    #[instrument(skip(self, surface), fields(surface = %surface))]
    pub async fn delete_dict_word(&self, surface: &str) -> Result<()> {
        debug!("Sending delete word user dict word request to voicevox");

        let word_uuid = if let Some(word_uuid_raw) = self.find_uuid_by_surface(surface).await? {
            word_uuid_raw
        } else {
            return Err(anyhow::anyhow!("Word not found"))
        };

        let user_dict_word_url = self.voicevox_url
            .join(format!("/user_dict_word/{}", word_uuid).as_str())
            .context("Failed to join URL")?;

        match self.voicevox_client.delete(user_dict_word_url).send().await {
            Ok(res) => {
                let status_ok = res.status().is_success();
                if status_ok {
                    info!("User dict word delete successfully");
                    Ok(())
                } else {
                    warn!("User dict word delete failed with status code {}", res.status());
                    Err(anyhow::anyhow!("User dict word delete failed with status code {}", res.status()))
                }
            }
            Err(e) => {
                error!("Failed to delete user dict word:\n{}", e);
                Err(anyhow::anyhow!("Failed to delete user dict word:\n{}", e))
            }
        }
    }

    #[instrument(skip(self, json_content))]
    pub async fn import_dict(&self, json_content: &str) -> Result<()> {
        debug!("Sending import user dict words request to voicevox");

        let mut user_dict_words_url = self.voicevox_url
            .join("/import_user_dict")
            .context("Failed to join URL")?;

        user_dict_words_url.query_pairs_mut()
            .append_pair("override", "true");

        match self.voicevox_client.post(user_dict_words_url).body(json_content.to_string()).send().await {
            Ok(res) => {
                let status_ok = res.status().is_success();
                if status_ok {
                    info!("User dict words import successfully");
                    Ok(())
                } else {
                    warn!("User dict words import failed with status code {}", res.status());
                    Err(anyhow::anyhow!("User dict words import failed with status code {}", res.status()))
                }
            }
            Err(e) => {
                error!("Failed to import user dict words:\n{}", e);
                Err(anyhow::anyhow!("Failed to import user dict words:\n{}", e))
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn reset_dict(&self) -> Result<()> {
        debug!("Sending reset user dict request to voicevox");

        let dict_content = self.get_user_dict().await?;
        let dict_json: Value = serde_json::from_str(&dict_content)?;

        let mut uuid_list = Vec::new();

        if let Some(dict_obj) = dict_json.as_object() {
            for (uuid, _value) in dict_obj {
                uuid_list.push(uuid.clone());
            }
        }

        for uuid in uuid_list {
            self.delete_dict_word(&uuid).await?;
        }

        Ok(())
    }
}