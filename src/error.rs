// use thiserror::Error;
//
// #[derive(Error, Debug)]
// pub enum BotError {
//     #[error("Database error: {0}")]
//     Database(#[from] sqlx::Error),
//
//     #[error("Voicevox error: {0}")]
//     Voicevox(String),
//
//     #[error("Discord API error: {0}")]
//     Discord(#[from] serenity::Error),
//
//     #[error("Configuration error: {0}")]
//     Config(String),
// }