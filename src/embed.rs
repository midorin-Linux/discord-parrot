use serenity::{builder::{CreateEmbed, CreateEmbedAuthor},  prelude::*};

pub async fn simple_embed(ctx: &Context, title: &str,description: &str, color: u32) -> CreateEmbed {
    match ctx.http.get_current_user().await {
        Ok(user) => {
            let embed = CreateEmbed::new()
                .author(CreateEmbedAuthor::new(user.display_name()).icon_url(user.avatar_url().unwrap_or_else(|| "https://cdn.discordapp.com/embed/avatars/0.png".to_string())))
                .title(title)
                .description(description)
                .color(color);
            embed
        },
        Err(why) => {
            tracing::warn!("Failed to get current user: {:?}", why);
            let embed = CreateEmbed::new()
                .title(title)
                .description(description)
                .color(color);
            embed
        },
    }
}