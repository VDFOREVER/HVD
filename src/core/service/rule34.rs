use crate::core::utils::{PostData, Utils};
use serde::Deserialize;

pub struct Rule34;

#[derive(Deserialize)]
pub struct Post {
    pub file_url: String,
    pub tags: String,
}

#[derive(Deserialize)]
pub struct Posts {
    pub post: Vec<Post>,
}

const RULE34: &str = "https://api.rule34.xxx/index.php?page=dapi&s=post&q=index&tags=";

impl Rule34 {
    pub async fn pasrse(tag: &String) -> Result<Vec<PostData>, String> {
        let request = Utils::request(format!("{}{}", RULE34, tag)).await;

        let xml = match request {
            Ok(xml) => xml,
            Err(message) => return Err(message.to_string()),
        };

        if xml == "<posts count=\"0\" offset=\"0\"/>" {
            return Err("empty".to_string());
        }

        let parse_result = match Self::parse(xml) {
            Ok(postdata) => postdata,
            Err(message) => return Err(message.to_string()),
        };

        Ok(parse_result)
    }

    fn parse(xml: String) -> Result<Vec<PostData>, String> {
        let result: std::result::Result<Posts, serde_xml_rs::Error> = serde_xml_rs::from_str(&xml);
        let pasre = match result {
            Ok(result) => result,
            Err(message) => return Err(message.to_string()),
        };

        let mut all_post: Vec<PostData> = vec![];
        for post in pasre.post {
            let tags = {
                let mut vec: Vec<String> = Vec::new();
                for tag in post.tags.split(' ') {
                    if tag.is_empty() {
                        continue;
                    }

                    vec.push(tag.to_string())
                }

                vec
            };

            all_post.push(PostData {
                content: vec![post.file_url],
                tags,
            })
        }

        Ok(all_post)
    }
}
