use crate::core::utils::{PostData, Utils};
use serde::Deserialize;

pub struct Kemono;

#[derive(Deserialize)]
pub struct Attachments {
    pub path: String,
}

#[derive(Deserialize)]
pub struct Post {
    pub attachments: Vec<Attachments>,
}

impl Kemono {
    pub async fn pasrse(url: &String) -> Result<Vec<PostData>, String> {
        let html = Utils::request(url.to_string()).await;
        let json = match html {
            Ok(html) => html,
            Err(message) => return Err(message.to_string()),
        };

        let parse_result = match Self::parse(json) {
            Ok(postdata) => postdata,
            Err(message) => return Err(message.to_string()),
        };

        Ok(parse_result)
    }

    fn parse(xml: String) -> Result<Vec<PostData>, String> {
        let result: std::result::Result<Vec<Post>, serde_json::Error> = serde_json::from_str(&xml);
        let mut pasre = match result {
            Ok(result) => result,
            Err(message) => return Err(message.to_string()),
        };

        let content = pasre
            .iter_mut()
            .flat_map(|x| {
                x.attachments
                    .iter_mut()
                    .map(|y| format!("https://kemono.su{}", y.path))
            })
            .collect();

        Ok(vec![PostData {
            content,
            tags: vec![],
        }])
    }
}
