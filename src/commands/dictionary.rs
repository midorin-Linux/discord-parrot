use crate::voice::voicevox::client::Client as VoicevoxClient;
use crate::embed;
use anyhow::Result;
use std::io::Write;
use serenity::{
    builder::{CreateCommand, CreateCommandOption},
    model::application::{CommandInteraction, CommandOptionType, CommandDataOptionValue},
    prelude::*,
    builder::CreateInteractionResponse,
    builder::CreateInteractionResponseFollowup,
};
use serde_json::Value;
use tracing::debug;

pub async fn run(ctx: &Context, interaction: &CommandInteraction, voicevox_client: &VoicevoxClient) -> Result<()> {
    interaction.defer(&ctx.http).await?;

    let response_embed = process_dictionary_command(ctx, interaction, voicevox_client).await;

    let builder = CreateInteractionResponseFollowup::new().embed(response_embed);

    let _ = interaction.create_followup(&ctx.http, builder).await?;

    Ok(())
}

async fn process_dictionary_command(ctx: &Context, interaction: &CommandInteraction, voicevox_client: &VoicevoxClient) -> serenity::all::CreateEmbed {
    let subcommand_name = match interaction.data.options().first() {
        Some(cmd) => cmd.name,
        None => {
            return embed::simple_embed(ctx, "エラー", "サブコマンドを指定してください。", 0xff0000).await;
        }
    };

    match subcommand_name.as_ref() {
        "add" => add_word(ctx, interaction, voicevox_client).await,
        "edit" => edit_word(ctx, interaction, voicevox_client).await,
        "list" => list_data(ctx, voicevox_client).await,
        "remove" => remove_word(ctx, interaction, voicevox_client).await,
        "reset" => reset_data(ctx, voicevox_client).await,
        "restore" => restore_data(ctx, voicevox_client).await,
        _ => embed::simple_embed(ctx, "エラー", &format!("「{}」は不明なコマンドです。", subcommand_name), 0xff0000).await,
    }
}

async fn add_word(ctx: &Context, interaction: &CommandInteraction, voicevox_client: &VoicevoxClient) -> serenity::all::CreateEmbed {
    debug!("Adding word to dictionary: {:?}", interaction.data.options);

    let subcommand_args = if let Some(CommandDataOptionValue::SubCommand(args)) =
        interaction.data.options.get(0).map(|opt| &opt.value)
    {
        args
    } else {
        return embed::simple_embed(ctx, "エラー", "サブコマンドの引数を正しく取得できませんでした。", 0xff0000).await;
    };

    let (surface, pronunciation, accent_type) = {
        let surface_option = subcommand_args
            .iter()
            .find(|opt| opt.name == "surface");
        let surface = match surface_option {
            Some(opt) => {
                if let CommandDataOptionValue::String(value) = &opt.value {
                    value
                } else {
                    return embed::simple_embed(ctx, "エラー", "'surface' オプションの値が文字列ではありません", 0xff0000).await;
                }
            }
            None => {
                return embed::simple_embed(ctx, "エラー", "'surface' オプションが見つかりません。", 0xff0000).await;
            }
        };

        let pronunciation_option = subcommand_args
            .iter()
            .find(|opt| opt.name == "pronunciation");
        let pronunciation = match pronunciation_option {
            Some(opt) => {
                if let CommandDataOptionValue::String(value) = &opt.value {
                    value
                } else {
                    return embed::simple_embed(ctx, "エラー", "'pronunciation' オプションの値が文字列ではありません", 0xff0000).await;
                }
            }
            None => {
                return embed::simple_embed(ctx, "エラー", "'pronunciation' オプションの値が見つかりません", 0xff0000).await;
            }
        };

        let accent_type_option = subcommand_args
            .iter()
            .find(|opt| opt.name == "accent_type");
        let accent_type = match accent_type_option {
            Some(opt) => {
                if let CommandDataOptionValue::Integer(value) = &opt.value {
                    value
                } else {
                    return embed::simple_embed(ctx, "エラー", "'accent_type' オプションの値が数字ではありません", 0xff0000).await;
                }
            }
            None => {
                return embed::simple_embed(ctx, "エラー", "'accent_type' オプションの値が見つかりません", 0xff0000).await;
            }
        };
        let accent_type_str = accent_type.to_string();

        (surface, pronunciation, accent_type_str)
    };

    if voicevox_client.find_uuid_by_surface(surface.as_ref()).await.is_ok() {
        return embed::simple_embed(ctx, "エラー", "既に辞書に同じ単語が存在します", 0xff0000).await;
    }

    match voicevox_client.add_dict_word(surface, pronunciation, accent_type.parse::<u8>().unwrap(), None).await {
        Ok(()) => {
            auto_save_data(voicevox_client).await.unwrap();
            let description = format!("**単語:** {}\n**読み方:** {}\n**アクセント:** {}", surface, pronunciation, accent_type);
            embed::simple_embed(ctx, "辞書に追加しました", &description, 0x00ff00).await
        },
        Err(e) => embed::simple_embed(ctx, "エラー", &format!("辞書の追加に失敗しました: {}", e), 0xff0000).await
    }
}

async fn edit_word(ctx: &Context, interaction: &CommandInteraction, voicevox_client: &VoicevoxClient) -> serenity::all::CreateEmbed {
    debug!("Editing word to dictionary: {:?}", interaction.data.options);

    let subcommand_args = if let Some(CommandDataOptionValue::SubCommand(args)) =
        interaction.data.options.get(0).map(|opt| &opt.value)
    {
        args
    } else {
        return embed::simple_embed(ctx, "エラー", "サブコマンドの引数を正しく取得できませんでした。", 0xff0000).await;
    };

    let (surface, pronunciation, accent_type) = {
        let surface_option = subcommand_args
            .iter()
            .find(|opt| opt.name == "surface");
        let surface = match surface_option {
            Some(opt) => {
                if let CommandDataOptionValue::String(value) = &opt.value {
                    value
                } else {
                    return embed::simple_embed(ctx, "エラー", "'surface' オプションの値が文字列ではありません", 0xff0000).await;
                }
            }
            None => {
                return embed::simple_embed(ctx, "エラー", "'surface' オプションが見つかりません。", 0xff0000).await;
            }
        };

        let pronunciation_option = subcommand_args
            .iter()
            .find(|opt| opt.name == "pronunciation");
        let pronunciation = match pronunciation_option {
            Some(opt) => {
                if let CommandDataOptionValue::String(value) = &opt.value {
                    value
                } else {
                    return embed::simple_embed(ctx, "エラー", "'pronunciation' オプションの値が文字列ではありません", 0xff0000).await;
                }
            }
            None => {
                return embed::simple_embed(ctx, "エラー", "'pronunciation' オプションの値が見つかりません", 0xff0000).await;
            }
        };

        let accent_type_option = subcommand_args
            .iter()
            .find(|opt| opt.name == "accent_type");
        let accent_type = match accent_type_option {
            Some(opt) => {
                if let CommandDataOptionValue::Integer(value) = &opt.value {
                    value
                } else {
                    return embed::simple_embed(ctx, "エラー", "'accent_type' オプションの値が数字ではありません", 0xff0000).await;
                }
            }
            None => {
                return embed::simple_embed(ctx, "エラー", "'accent_type' オプションの値が見つかりません", 0xff0000).await;
            }
        };
        let accent_type_str = accent_type.to_string();

        (surface, pronunciation, accent_type_str)
    };

    if !voicevox_client.find_uuid_by_surface(surface.as_ref()).await.is_ok() {
        return embed::simple_embed(ctx, "エラー", &format!("単語「{}」は辞書に登録されていません", surface), 0xff0000).await;
    }

    match voicevox_client.rewrite_dict_word(surface, pronunciation, accent_type.parse::<u8>().unwrap(), None).await {
        Ok(()) => {
            auto_save_data(voicevox_client).await.unwrap();
            let description = format!("**単語:** {}\n**読み方:** {}\n**アクセント:** {}", surface, pronunciation, accent_type);
            embed::simple_embed(ctx, "単語を編集しました", &description, 0x00ff00).await
        },
        Err(e) => embed::simple_embed(ctx, "エラー", &format!("辞書内の単語の編集に失敗しました: {}", e), 0xff0000).await
    }
}

async fn list_data(ctx: &Context, voicevox_client: &VoicevoxClient) -> serenity::all::CreateEmbed {
    debug!("Listing dictionary data");

    match voicevox_client.get_user_dict().await {
        Ok(data) => {
            match serde_json::from_str::<Value>(&data) {
                Ok(json_data) => {
                    let mut formatted_entries = Vec::new();

                    if let Value::Object(dict) = json_data {
                        let total_entries = dict.len();

                        for (_uuid, entry) in dict.iter() {
                            if let Value::Object(word_data) = entry {
                                let surface = word_data.get("surface")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("不明");
                                let pronunciation = word_data.get("pronunciation")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("不明");
                                let accent_type = word_data.get("accent_type")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);

                                formatted_entries.push(format!("**{}** → {} (アクセント: {})",
                                                               surface, pronunciation, accent_type));
                            }
                        }

                        let max_entries = 20;
                        let display_entries = if formatted_entries.len() > max_entries {
                            let mut truncated = formatted_entries.into_iter().take(max_entries).collect::<Vec<_>>();
                            truncated.push(format!("... 他{}件", total_entries - max_entries));
                            truncated
                        } else {
                            formatted_entries
                        };

                        if display_entries.is_empty() {
                            embed::simple_embed(ctx, "辞書データ一覧", "辞書に登録されている単語はありません", 0x0099ff).await
                        } else {
                            let description = format!("**登録単語数:** {}件\n\n{}",
                                                      total_entries,
                                                      display_entries.join("\n")
                            );

                            let final_description = if description.len() > 2000 {
                                format!("**登録単語数:** {}件\n\n登録単語が多すぎるため、詳細な一覧を表示できません。\n`/dictionary remove` で不要な単語を削除してください。", total_entries)
                            } else {
                                description
                            };

                            embed::simple_embed(ctx, "辞書データ一覧", &final_description, 0x0099ff).await
                        }
                    } else {
                        embed::simple_embed(ctx, "辞書データ一覧", "辞書に登録されている単語はありません", 0x0099ff).await
                    }
                }
                Err(_) => {
                    let truncated_data = if data.len() > 1900 {
                        format!("{}...\n\n*データが長すぎるため一部省略されました*", &data[..1900])
                    } else {
                        data
                    };
                    embed::simple_embed(ctx, "辞書データ一覧", &format!("```json\n{}\n```", truncated_data), 0x0099ff).await
                }
            }
        },
        Err(e) => embed::simple_embed(ctx, "エラー", &format!("辞書の取得に失敗しました: {}", e), 0xff0000).await
    }
}

async fn remove_word(ctx: &Context, interaction: &CommandInteraction, voicevox_client: &VoicevoxClient) -> serenity::all::CreateEmbed {
    debug!("Removing word from dictionary: {:?}", interaction.data.options);

    let subcommand_args = if let Some(CommandDataOptionValue::SubCommand(args)) =
        interaction.data.options.get(0).map(|opt| &opt.value)
    {
        args
    } else {
        return embed::simple_embed(ctx, "エラー", "サブコマンドの引数を正しく取得できませんでした。", 0xff0000).await;
    };

    let surface_option = subcommand_args
        .iter()
        .find(|opt| opt.name == "surface");
    let surface = match surface_option {
        Some(opt) => {
            if let CommandDataOptionValue::String(value) = &opt.value {
                value
            } else {
                return embed::simple_embed(ctx, "エラー", "'surface' オプションの値が文字列ではありません", 0xff0000).await;
            }
        }
        None => {
            return embed::simple_embed(ctx, "エラー", "'surface' オプションが見つかりません。", 0xff0000).await;
        }
    };

    if !voicevox_client.find_uuid_by_surface(surface.as_ref()).await.is_ok() {
        return embed::simple_embed(ctx, "エラー", &format!("単語「{}」は辞書に登録されていません", surface), 0xff0000).await;
    }

    match voicevox_client.delete_dict_word(surface).await {
        Ok(()) => {
            auto_save_data(voicevox_client).await.unwrap();
            embed::simple_embed(ctx, "単語を削除しました", &format!("**削除した単語:** {}", surface), 0x00ff00).await
        },
        Err(e) => embed::simple_embed(ctx, "エラー", &format!("単語の削除に失敗しました: {}", e), 0xff0000).await
    }
}

async fn reset_data(ctx: &Context, voicevox_client: &VoicevoxClient) -> serenity::all::CreateEmbed {
    debug!("Resetting dictionary data");

    match voicevox_client.reset_dict().await {
        Ok(()) => {
            auto_save_data(voicevox_client).await.unwrap();
            embed::simple_embed(ctx, "辞書をリセットしました", "すべての単語が削除されました", 0x00ff00).await
        },
        Err(e) => embed::simple_embed(ctx, "エラー", &format!("辞書のリセットに失敗しました: {}", e), 0xff0000).await
    }
}

async fn restore_data(ctx: &Context, voicevox_client: &VoicevoxClient) -> serenity::all::CreateEmbed {
    debug!("Restoring dictionary data");

    let data = match std::fs::read_to_string("user_dict.json") {
        Ok(data) => data,
        Err(e) => {
            return embed::simple_embed(ctx, "エラー", &format!("辞書ファイルの読み込みに失敗しました: {}", e), 0xff0000).await;
        }
    };

    match voicevox_client.import_dict(data.as_str()).await {
        Ok(()) => {
            auto_save_data(voicevox_client).await.unwrap();
            embed::simple_embed(ctx, "辞書の復元に成功しました", "最後に保存されたデータから復元されました", 0x00ff00).await
        }
        Err(e) => embed::simple_embed(ctx, "エラー", &format!("辞書の復元に失敗しました。再度実行してください: {}", e), 0xff0000).await
    }
}

async fn auto_save_data(voicevox_client: &VoicevoxClient) -> Result<()> {
    match voicevox_client.get_user_dict().await {
        Ok(data) => {
            let mut file = std::fs::File::create("user_dict.json")?;
            file.write_all(data.as_bytes())?;
            Ok(())
        }
        Err(e) => {
            Err(anyhow::anyhow!("Failed to auto save dictionary: {}", e))
        }
    }
}

pub fn register() -> CreateCommand {
    let command = CreateCommand::new("dictionary");
    command
        .description("VOICEVOXの辞書を管理します")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "add", "辞書に単語を追加します")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "surface", "追加する単語")
                        .required(true)
                        .max_length(100)
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "pronunciation", "カタカナでの読み方")
                        .required(true)
                        .max_length(100)
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Integer, "accent_type", "何文字目にアクセントを付けるか")
                        .required(true)
                )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "edit", "辞書にある単語の編集をします")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "surface", "編集する単語")
                        .required(true)
                        .max_length(100)
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "pronunciation", "カタカナでの読み方")
                        .required(true)
                        .max_length(100)
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Integer, "accent_type", "何文字目にアクセントを付けるか")
                        .required(true)
                )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "辞書にある単語の一覧を表示します")
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "remove", "辞書の単語を削除します")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "surface", "削除する単語")
                        .required(true)
                )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "reset", "辞書をリセットします")
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "restore", "最後に保存された辞書データを復元します")
        )
}