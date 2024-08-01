use super::db::{Db, Service, Services};
use log::{info, warn};
use reqwest::Client;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use teloxide::{
    prelude::Requester,
    types::{ChatId, InputFile, InputMedia, InputMediaDocument, Message},
    Bot, RequestError,
};
use tokio::time::sleep;

pub struct PostData {
    pub content: Vec<String>,
    pub tags: Vec<String>,
}

pub struct Utils;

impl Utils {
    pub fn remove_dub(vec1: &mut Vec<String>, vec2: &[String]) {
        vec1.retain(|e| !vec2.contains(e));
    }

    pub fn exist_in_array(vec1: &[String], vec2: &[String]) -> bool {
        vec1.iter().any(|x| vec2.iter().any(|y| x == y))
    }

    pub async fn send_list_tag(
        pool: &Pool<Sqlite>,
        bot: &Bot,
        chat_id: i64,
    ) -> Result<Message, RequestError> {
        let user_rule = Db::fetch_all_user(pool, &Services::Rule34).await;
        let user_gelbooru = Db::fetch_all_user(pool, &Services::Gelbooru).await;
        let user_kemono = Db::fetch_all_user(pool, &Services::Kemono).await;
        let rule34 = user_rule.iter().find(|x| x.user_id == chat_id);
        let gelbooru = user_gelbooru.iter().find(|x| x.user_id == chat_id);
        let kemono = user_kemono.iter().find(|x| x.user_id == chat_id);

        let chatid = ChatId(chat_id);

        if rule34.is_none() || gelbooru.is_none() || kemono.is_none() {
            return bot.send_message(chatid, "empty").await;
        }

        let rule34 = rule34.unwrap();
        let gelbooru = gelbooru.unwrap();
        let kemono = kemono.unwrap();

        let rule34_tags = serde_json::from_str::<Vec<String>>(&rule34.tags)
            .unwrap_or_default()
            .join("\n");
        let rule34_antitags = serde_json::from_str::<Vec<String>>(&rule34.antitags)
            .unwrap_or_default()
            .join("\n");
        let gelbooru_tags = serde_json::from_str::<Vec<String>>(&gelbooru.tags)
            .unwrap_or_default()
            .join("\n");
        let gelbooru_antitags = serde_json::from_str::<Vec<String>>(&gelbooru.antitags)
            .unwrap_or_default()
            .join("\n");
        let kemono_tags = serde_json::from_str::<Vec<String>>(&kemono.tags)
            .unwrap_or_default()
            .join("\n");

        let message = format!(
            "-----Rule34 Tag-----\n{}\n-----Rule34 Anti tag-----\n{}\n -----Gelbooru Tag-----\n{}\n-----Gelbooru Anti tag-----\n{}\n-----Kemono Tag-----\n{}\n",
            rule34_tags, rule34_antitags, gelbooru_tags, gelbooru_antitags, kemono_tags
        );

        bot.send_message(chatid, message).await
    }

    pub async fn repeat_tags(user: &[Service]) -> HashMap<String, Vec<i64>> {
        let mut repeat_tags: HashMap<String, Vec<i64>> = HashMap::new();
        user.iter().filter(|x| !x.tags.is_empty()).for_each(|user| {
            serde_json::from_str::<Vec<String>>(&user.tags)
                .unwrap()
                .iter()
                .for_each(|tag| {
                    if let Some(vec) = repeat_tags.get_mut(tag) {
                        vec.push(user.user_id)
                    } else {
                        repeat_tags.insert(tag.to_string(), vec![user.user_id]);
                    }
                });
        });

        repeat_tags
    }

    pub async fn send_image_group(bot: &Bot, chat_id: i64, photos: Vec<String>) {
        let chatid = ChatId(chat_id);
        let mut tmp = vec![];
        let mut counter = 0;

        for photo in &photos {
            tmp.push(photo);
            counter += 1;

            if tmp.len() != 10 && (photos.len() - counter as usize) != 0 {
                continue;
            }

            let mut img = vec![];
            for sss in &tmp {
                let image = InputFile::url(url::Url::parse(sss).unwrap());
                let media = InputMedia::Document(InputMediaDocument::new(image));
                img.push(media);
            }

            let send = bot.send_media_group(chatid, img).await;
            match send {
                Ok(_) => {
                    for content in &photos {
                        info!("{}", format!("Send: {:#?}", content));
                    }
                }
                Err(message) => {
                    warn!("{}", message);
                    for sss in &tmp {
                        bot.send_message(chatid, *sss).await.unwrap();
                        sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            }

            tmp.clear();
            sleep(std::time::Duration::from_secs(10)).await;
        }
    }

    pub async fn request(url: String) -> Result<String, Box<reqwest::Error>> {
        Ok(Client::new()
            .get(url)
            .timeout(std::time::Duration::from_secs(10))
            .header(
                reqwest::header::COOKIE,
                format!("{}={}", "fringeBenefits", "yup"),
            )
            .send()
            .await?
            .text()
            .await?)
    }
}
