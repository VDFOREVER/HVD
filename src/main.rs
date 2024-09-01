use log::{error, info};
use sqlx::{Pool, Sqlite};
use std::{env::var, process::exit};
use teloxide::{prelude::*, utils::command::BotCommands};
use tg_bot::core::{
    db::{Db, Services},
    service::{gelbooru::Gelbooru, kemono::Kemono, pixiv::Pixiv, rule34::Rule34},
    utils::Utils,
};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    dotenv::from_path(".env").expect("error loading env");

    let mut pixiv_login = Pixiv::login().await.unwrap();

    let bot = Bot::from_env();
    let bot_clone = bot.clone();

    ctrlc::set_handler(|| {
        exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let handle = tokio::spawn(async move {
        loop {
            let pool = Db::open().await.unwrap();
            pixiv_login = pixiv_login.refresh().await.unwrap();

            for service in [
                Services::Rule34,
                Services::Gelbooru,
                Services::Kemono,
                Services::Pixiv,
            ] {
                info!("Start {:?}", service);

                let user_results = Db::fetch_all_user(&pool, &service).await;
                let repeat_tags = Utils::repeat_tags(&user_results).await;

                for (repeat_tag, user_id) in repeat_tags {
                    let post = match service {
                        Services::Rule34 => Rule34::pasrse(&repeat_tag).await,
                        Services::Gelbooru => Gelbooru::pasrse(&repeat_tag).await,
                        Services::Kemono => Kemono::pasrse(&repeat_tag).await,
                        Services::Pixiv => {
                            Pixiv::pasrse(&repeat_tag, pixiv_login.access_token.clone()).await
                        }
                    };

                    let post = match post {
                        Ok(post) => post,
                        Err(message) => {
                            error!("{}", message);
                            continue;
                        }
                    };

                    for user in user_id {
                        let find_user = user_results
                            .clone()
                            .into_iter()
                            .find(|x| x.user_id == user)
                            .unwrap();

                        let post_history = serde_json::from_str::<Vec<String>>(&find_user.history)
                            .unwrap_or_default();

                        let antitags = serde_json::from_str::<Vec<String>>(&find_user.antitags)
                            .unwrap_or_default();

                        let mut send = vec![];

                        for data in &post {
                            for content in &data.content {
                                if Utils::exist_in_array(&antitags, &data.tags)
                                    || post_history.contains(content)
                                {
                                    continue;
                                }

                                send.push(content.clone());
                            }
                        }

                        Utils::send_image_group(&bot_clone, user, send.clone()).await;

                        Db::add_history(&pool, user, send, &service).await.unwrap();
                    }

                    sleep(std::time::Duration::from_secs(1)).await
                }
            }

            pool.close().await;
            info!("Sleep 60m");
            sleep(std::time::Duration::from_secs(60 * 60)).await
        }
    });

    let handle2 = tokio::spawn(async move {
        Command::repl(bot, answer).await;
    });

    handle2.await.unwrap();
    handle.await.unwrap();

    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Add tag.", parse_with = "split")]
    AddTag {
        service: String,
        tag: String,
    },
    #[command(description = "Add antitag.", parse_with = "split")]
    AddAntiTag {
        service: String,
        tag: String,
    },
    #[command(description = "Add delete tag.", parse_with = "split")]
    RmTag {
        service: String,
        tag: String,
    },
    #[command(description = "Add delete antitag.", parse_with = "split")]
    RmAntiTag {
        service: String,
        tag: String,
    },
    AddUser {
        user_id: i64,
    },
    RmUser {
        user_id: i64,
    },
    #[command(description = "List tags.", parse_with = "split")]
    TagList,
}

const HELP: &str = "/help
/addtag {service} {tag}
/rmtag {service} {tag}
/addantitag {service} {tag}
/rmantitag {service} {tag}
/taglist


service:
    rule34
    gelbooru
    kemono
    pixiv
";

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let pool: Pool<Sqlite> = Db::open().await?;

    match cmd {
        Command::Help => bot.send_message(msg.chat.id, HELP).await?,
        Command::AddTag { service, tag } => {
            let restult =
                Db::add_tag(&pool, &tag, msg.chat.id.0, &Db::string_toservice(service)).await;
            match restult {
                Ok(()) => {
                    bot.send_message(msg.chat.id, "Add tag!".to_string())
                        .await?
                }
                Err(message) => {
                    bot.send_message(msg.chat.id, format!("Error add {}: {}", tag, message))
                        .await?
                }
            }
        }
        Command::AddAntiTag { service, tag } => {
            let restult =
                Db::add_antitag(&pool, &tag, msg.chat.id.0, &Db::string_toservice(service)).await;
            match restult {
                Ok(()) => {
                    bot.send_message(msg.chat.id, "Add antitag!".to_string())
                        .await?
                }
                Err(message) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Error add AntiTag {}: {}", tag, message),
                    )
                    .await?
                }
            }
        }
        Command::RmTag { service, tag } => {
            Db::rm_tag(&pool, tag, msg.chat.id.0, &Db::string_toservice(service))
                .await
                .unwrap();
            bot.send_message(msg.chat.id, "remove tag!".to_string())
                .await?
        }
        Command::RmAntiTag { service, tag } => {
            Db::rm_antitag(&pool, tag, msg.chat.id.0, &Db::string_toservice(service))
                .await
                .unwrap();
            bot.send_message(msg.chat.id, "remove antitag!".to_string())
                .await?
        }
        Command::TagList => Utils::send_list_tag(&pool, &bot, msg.chat.id.0).await?,

        Command::AddUser { user_id } => {
            if msg.chat.id.0
                != var("ADMIN")
                    .expect("error loading env")
                    .parse::<i64>()
                    .unwrap()
            {
                bot.send_message(msg.chat.id, "Not access".to_string())
                    .await?
            } else {
                Db::create_user_is_not_exitst(&pool, user_id, &Services::Rule34).await?;
                Db::create_user_is_not_exitst(&pool, user_id, &Services::Gelbooru).await?;
                Db::create_user_is_not_exitst(&pool, user_id, &Services::Kemono).await?;
                Db::create_user_is_not_exitst(&pool, user_id, &Services::Pixiv).await?;

                bot.send_message(msg.chat.id, "Add User".to_string())
                    .await?
            }
        }
        Command::RmUser { user_id } => {
            if msg.chat.id.0
                != var("ADMIN")
                    .expect("error loading env")
                    .parse::<i64>()
                    .unwrap()
            {
                bot.send_message(msg.chat.id, "Not access".to_string())
                    .await?
            } else {
                Db::remove_user(&pool, user_id, &Services::Rule34).await?;
                Db::remove_user(&pool, user_id, &Services::Gelbooru).await?;
                Db::remove_user(&pool, user_id, &Services::Kemono).await?;
                Db::remove_user(&pool, user_id, &Services::Pixiv).await?;

                bot.send_message(msg.chat.id, "Remove User".to_string())
                    .await?
            }
        }
    };

    pool.close().await;
    Ok(())
}
