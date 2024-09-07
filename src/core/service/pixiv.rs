use crate::core::utils::PostData;
use oauth2_utils::urlsafe::{b64::urlsafe_b64encode, urlsafe_token};
use reqwest::{header::HeaderMap, Client};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, io};

pub struct Pixiv {
    pub access_token: String,
}

#[derive(Deserialize, Debug)]
pub struct ImageUrl {
    pub original: String,
}

#[derive(Deserialize, Debug)]
pub struct MetaPage {
    pub image_urls: ImageUrl,
}

#[derive(Deserialize, Debug)]
pub struct MetaSinglePage {
    pub original_image_url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Illusts {
    pub id: i64,
    pub meta_single_page: MetaSinglePage,
    pub meta_pages: Vec<MetaPage>,
}

#[derive(Deserialize, Debug)]
pub struct Post {
    pub illusts: Vec<Illusts>,
}

const USER_AGENT: &str = "PixivAndroidApp/5.0.234 (Android 11; Pixel 5)";
const REDIRECT_URI: &str = "https://app-api.pixiv.net/web/v1/users/auth/pixiv/callback";
const LOGIN_URL: &str = "https://app-api.pixiv.net/web/v1/login";
const CLIENT_ID: &str = "MOBrBDS8blbauoSck0ZfDbtuzpyT";
const CLIENT_SECRET: &str = "lsACyCD94FhDUtGTXi3QzcFE2uU1hqtDaKeqrdwj";
const AUTH_TOKEN_URL: &str = "https://oauth.secure.pixiv.net/auth/token";
const PIXIV: &str = "https://app-api.pixiv.net/v1/user/illusts?&type=illust&offset=0&user_id=";

impl Pixiv {
    pub async fn login() -> Result<Self, Box<reqwest::Error>> {
        let code_verifier = urlsafe_token(32);
        let code_challenge = urlsafe_b64encode(Sha256::digest(&code_verifier));
        let code_challenge = code_challenge.trim_end_matches('=');
        println!(
            "{}?code_challenge={}&code_challenge_method=S256&client=pixiv-android",
            LOGIN_URL, code_challenge
        );

        let mut code = String::new();
        io::stdin().read_line(&mut code).unwrap();

        let mut form = Self::form();
        form.insert("code".into(), code.trim_end_matches('\n').into());
        form.insert(
            "code_verifier".into(),
            code_verifier.trim_end_matches('\n').into(),
        );
        form.insert("grant_type".into(), "authorization_code".into());
        form.insert("redirect_uri".into(), REDIRECT_URI.into());

        let res = Client::new()
            .post(AUTH_TOKEN_URL)
            .form(&form)
            .headers(Self::header())
            .send()
            .await?
            .text()
            .await?;

        let (access_token, refresh_token) = Self::token_extract(res);
        Self::save_refresh_token(refresh_token);

        Ok(Self { access_token })
    }

    pub async fn refresh() -> Result<Self, Box<reqwest::Error>> {
        let mut form = Self::form();
        form.insert("grant_type".into(), "refresh_token".into());
        form.insert(
            "refresh_token".into(),
            Self::read_refresh_token().trim_end_matches('\n').into(),
        );

        let res = Client::new()
            .post(AUTH_TOKEN_URL)
            .form(&form)
            .headers(Self::header())
            .send()
            .await?
            .text()
            .await?;

        let (access_token, refresh_token) = Self::token_extract(res);
        Self::save_refresh_token(refresh_token);

        Ok(Self { access_token })
    }

    pub async fn pasrse(tag: &String, token: String) -> Result<Vec<PostData>, String> {
        let html = Self::request(format!("{}{}", PIXIV, tag), token).await;
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
        let result: std::result::Result<Post, serde_json::Error> = serde_json::from_str(&xml);
        let pasre = match result {
            Ok(result) => result,
            Err(message) => return Err(message.to_string()),
        };

        let originals: Vec<String> = pasre
            .illusts
            .iter()
            .flat_map(|illust| {
                if illust.meta_pages.is_empty() {
                    if let Some(url) = &illust.meta_single_page.original_image_url {
                        if !url.is_empty() {
                            return vec![url.clone()];
                        }
                    }
                    Vec::new()
                } else {
                    illust
                        .meta_pages
                        .iter()
                        .map(|page| page.image_urls.original.clone())
                        .collect::<Vec<String>>()
                }
            })
            .collect();

        Ok(vec![PostData {
            content: originals,
            tags: vec![],
        }])
    }

    fn form() -> HashMap<String, String> {
        let mut form = HashMap::new();
        form.insert("client_id".into(), CLIENT_ID.into());
        form.insert("client_secret".into(), CLIENT_SECRET.into());
        form.insert("include_policy".into(), "true".into());

        form
    }

    fn header() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", USER_AGENT.parse().unwrap());

        headers
    }

    async fn request(url: String, token: String) -> Result<String, Box<reqwest::Error>> {
        Ok(Client::new()
            .get(url)
            .headers(Self::header())
            .bearer_auth(token)
            .send()
            .await?
            .text()
            .await?)
    }

    fn token_extract(data: String) -> (String, String) {
        let json: Value = serde_json::from_str(&data).unwrap_or_default();
        let access_token = json
            .get("response")
            .unwrap()
            .get("access_token")
            .unwrap()
            .to_string()
            .replace("\\", "")
            .trim_matches('"')
            .to_string();

        let refresh_token = json
            .get("response")
            .unwrap()
            .get("refresh_token")
            .unwrap()
            .to_string()
            .replace("\\", "")
            .trim_matches('"')
            .to_string();

        (access_token, refresh_token)
    }

    fn save_refresh_token(token: String) {
        fs::write("tmp", token).expect("Unable to write file")
    }

    fn read_refresh_token() -> String {
        fs::read_to_string("tmp").expect("Unable to read file")
    }
}
