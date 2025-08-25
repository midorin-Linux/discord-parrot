use crate::config::Config;
use crate::voice::voicevox::audio::Audio;
use crate::voice::voicevox::dictionary::Dictionary;
use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde_json::{Value, json};
use tracing::{debug, info, warn, error, instrument};
use url::Url;

pub struct Client {
    audio: Audio,
    dictionary: Dictionary,
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
        
        let audio = Audio::new(voicevox_client.clone(), voicevox_url.clone())?;
        
        let dictionary = Dictionary::new(voicevox_client.clone(), voicevox_url.clone())?;

        debug!("Voicevox handler initialized");

        Ok(Self {
            audio,
            dictionary,
            voicevox_client,
            voicevox_url,
        })
    }
}