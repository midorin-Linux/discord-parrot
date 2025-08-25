use serenity::prelude::Context;
use serenity::all::{Message, GuildId, User};
use regex::Regex;
use once_cell::sync::Lazy;

static RE_EMOJI: Lazy<Regex> = Lazy::new(|| Regex::new(r"<a?:\w+:\d+>").unwrap());
static RE_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://[\w!?/+\-_~;.,*&@#$%()='\]]+").unwrap());

pub async fn format_voicevox_message(ctx: &Context, msg: &Message) -> String {
    let mut text = msg.content.clone();

    if let Some(guild_id) = msg.guild_id {
        text = replace_user_mentions(ctx, guild_id, &msg.mentions, &text).await;
    }

    text = RE_EMOJI.replace_all(&text, "").to_string();
    text = RE_URL.replace_all(&text, "URL、").to_string();

    if !msg.attachments.is_empty() {
        if text.trim().is_empty() {
            text = "添付ファイル".to_string();
        } else {
            text.insert_str(0, "添付ファイル、");
        }
    }

    text
}

async fn replace_user_mentions(
    ctx: &Context,
    guild_id: GuildId,
    mentions: &[User],
    text: &str,
) -> String {
    let mut result_text = text.to_string();

    for user in mentions {
        let display_name = match guild_id.member(&ctx.http, user.id).await {
            Ok(member) => member.nick.unwrap_or_else(|| user.name.clone()),
            Err(_) => user.name.clone(),
        };

        let pattern = format!(r"<@!?{}>", user.id);

        if let Ok(re) = Regex::new(&pattern) {
            result_text = re.replace_all(&result_text, format!("アットマーク{}、", display_name)).to_string();
        }
    }

    result_text
}