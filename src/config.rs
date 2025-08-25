use anyhow::Result;
use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::{Deserialize, Deserializer};
use std::str::FromStr;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(rename = "DATABASE_URL")]
    pub database_url: String,

    #[serde(rename = "DISCORD_TOKEN")]
    pub discord_token: String,

    #[serde(rename = "GUILD_ID")]
    pub guild_id: String,

    #[serde(rename = "VOICEVOX_URL")]
    #[serde(deserialize_with = "deserialize_url")]
    pub voicevox_url: Url,

    #[serde(default = "default_speaker_id")]
    pub default_speaker_id: u8,

    #[serde(default = "default_speed_scale")]
    pub default_speed_scale: f64,

    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,
}

fn default_speaker_id() -> u8 { 1 }
fn default_speed_scale() -> f64 { 1.0 }
fn default_timeout() -> u64 { 10 }

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Url::from_str(&s).map_err(serde::de::Error::custom)
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // .envファイルが存在する場合は読み込む（エラーは無視）
        let _ = dotenvy::dotenv();

        let config = ConfigBuilder::builder()
            // .envファイルから読み込み（存在しない場合はスキップ）
            .add_source(
                File::with_name(".env")
                    .format(config::FileFormat::Ini)
                    .required(false)
            )
            // 環境変数から読み込み（優先度高）
            .add_source(Environment::default())
            .build()?;

        config.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.discord_token.is_empty() {
            return Err("Discord token cannot be empty".to_string());
        }

        if self.guild_id.parse::<u64>().is_err() {
            return Err("Guild ID must be a valid number".to_string());
        }

        if self.default_speed_scale <= 0.0 || self.default_speed_scale > 2.0 {
            return Err("Speed scale must be between 0.0 and 2.0".to_string());
        }

        Ok(())
    }
}