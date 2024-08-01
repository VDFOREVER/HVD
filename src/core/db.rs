use super::{
    service::{gelbooru::Gelbooru, kemono::Kemono, rule34::Rule34},
    utils::Utils,
};
use log::info;
use sqlx::{migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};
use tokio::time::sleep;

pub struct Db;

const DB_URL: &str = "sqlite://sqlite.db";

#[derive(Clone, sqlx::FromRow)]
pub struct Service {
    pub user_id: i64,
    pub tags: String,
    pub antitags: String,
    pub history: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Services {
    Rule34,
    Gelbooru,
    Kemono,
}

impl Db {
    pub async fn open() -> std::io::Result<Pool<Sqlite>> {
        if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
            match Sqlite::create_database(DB_URL).await {
                Ok(_) => info!("Create db success"),
                Err(error) => panic!("error: {}", error),
            }
        }
        let db = SqlitePool::connect(DB_URL)
            .await
            .expect("Error connect to db");

        sqlx::query("CREATE TABLE IF NOT EXISTS rule34 (id INTEGER PRIMARY KEY, user_id INTEGER, tags TEXT, antitags TEXT, history TEXT);").execute(&db).await.expect("Error create table");
        sqlx::query("CREATE TABLE IF NOT EXISTS gelbooru (id INTEGER PRIMARY KEY, user_id INTEGER, tags TEXT, antitags TEXT, history TEXT);").execute(&db).await.expect("Error create table");
        sqlx::query("CREATE TABLE IF NOT EXISTS kemono (id INTEGER PRIMARY KEY, user_id INTEGER, tags TEXT, antitags TEXT, history TEXT);").execute(&db).await.expect("Error create table");

        Ok(db)
    }

    fn service_tostring(service: &Services) -> &'static str {
        match service {
            Services::Gelbooru => "gelbooru",
            Services::Rule34 => "rule34",
            Services::Kemono => "kemono",
        }
    }

    pub fn string_toservice(service: String) -> Services {
        match service.as_str() {
            "gelbooru" => Services::Gelbooru,
            "rule34" => Services::Rule34,
            "kemono" => Services::Kemono,
            _ => Services::Rule34,
        }
    }

    pub async fn fetch_all_user(pool: &Pool<Sqlite>, service: &Services) -> Vec<Service> {
        let query = format!("SELECT * FROM {}", Self::service_tostring(service));
        sqlx::query_as::<_, Service>(&query)
            .fetch_all(pool)
            .await
            .unwrap()
    }

    pub async fn exist_user(
        pool: &Pool<Sqlite>,
        user_id: i64,
        service: &Services,
    ) -> std::io::Result<bool> {
        let user_results = Self::fetch_all_user(pool, service).await;

        Ok(user_results.iter().any(|i| i.user_id == user_id))
    }

    pub async fn create_user_is_not_exitst(
        pool: &Pool<Sqlite>,
        user_id: i64,
        service: &Services,
    ) -> std::io::Result<()> {
        let service_str = Self::service_tostring(service);

        if !Self::exist_user(pool, user_id, service).await? {
            let query_create = format!("INSERT INTO {} (user_id) VALUES (?)", &service_str);
            sqlx::query(&query_create)
                .bind(user_id)
                .execute(pool)
                .await
                .unwrap();
        }

        Ok(())
    }

    pub async fn remove_user(
        pool: &Pool<Sqlite>,
        user_id: i64,
        service: &Services,
    ) -> std::io::Result<()> {
        let service_str = Self::service_tostring(service);

        if Self::exist_user(pool, user_id, service).await? {
            let query_create = format!("DELETE FROM {} WHERE user_id=?", &service_str);
            sqlx::query(&query_create)
                .bind(user_id)
                .execute(pool)
                .await
                .unwrap();
        }

        Ok(())
    }

    async fn update_data(
        pool: &Pool<Sqlite>,
        new_data: String,
        table: String,
        user_id: i64,
        service: &Services,
    ) {
        let query_update = format!(
            "UPDATE {} SET {}=? WHERE user_id=?",
            Self::service_tostring(service),
            table
        );
        sqlx::query(&query_update)
            .bind(new_data)
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
    }

    pub async fn add_tag(
        pool: &Pool<Sqlite>,
        tag: &String,
        user_id: i64,
        service: &Services,
    ) -> core::result::Result<(), String> {
        if !Self::exist_user(pool, user_id, service).await.unwrap() {
            return Err("User not exist".to_string());
        }

        let user_results = Self::fetch_all_user(pool, service).await;
        let user = user_results.iter().find(|x| x.user_id == user_id).unwrap();

        let mut db_tags = serde_json::from_str::<Vec<String>>(&user.tags).unwrap_or_default();

        if db_tags.iter().any(|i| *i == *tag) {
            return Err("Tag exist".to_string());
        }

        db_tags.push(tag.clone());
        let new_tags = serde_json::to_string(&db_tags).unwrap();

        sleep(std::time::Duration::from_secs(5)).await;

        let posts = match service {
            Services::Rule34 => Rule34::pasrse(tag).await,
            Services::Gelbooru => Gelbooru::pasrse(tag).await,
            Services::Kemono => Kemono::pasrse(tag).await,
        };

        let posts = match posts {
            Ok(post) => post.into_iter().flat_map(|x| x.content).collect(),
            Err(message) => return Err(message),
        };

        Self::update_data(pool, new_tags, "tags".to_string(), user_id, service).await;
        Self::add_history(pool, user_id, posts, service)
            .await
            .unwrap();

        Ok(())
    }

    pub async fn rm_tag(
        pool: &Pool<Sqlite>,
        tag: String,
        user_id: i64,
        service: &Services,
    ) -> Result<(), String> {
        if !Self::exist_user(pool, user_id, service).await.unwrap() {
            return Err("User not exist".to_string());
        }

        let user_results = Self::fetch_all_user(pool, service).await;
        let user = user_results.iter().find(|x| x.user_id == user_id).unwrap();

        let mut db_tags = serde_json::from_str::<Vec<String>>(&user.tags).unwrap_or_default();

        if !db_tags.iter().any(|i| *i == tag) {
            return Err("Tag not found".to_string());
        }

        db_tags.retain(|x| *x != tag);

        let new_tags = serde_json::to_string(&db_tags).unwrap();

        Self::update_data(pool, new_tags, "tags".to_string(), user_id, service).await;

        Ok(())
    }

    pub async fn add_antitag(
        pool: &Pool<Sqlite>,
        tag: &String,
        user_id: i64,
        service: &Services,
    ) -> core::result::Result<(), String> {
        if !Self::exist_user(pool, user_id, service).await.unwrap() {
            return Err("User not exist".to_string());
        }

        let user_results = Self::fetch_all_user(pool, service).await;
        let user = user_results.iter().find(|x| x.user_id == user_id).unwrap();

        let mut db_tags = serde_json::from_str::<Vec<String>>(&user.antitags).unwrap_or_default();

        if db_tags.iter().any(|i| *i == *tag) {
            return Err("Tag exist".to_string());
        }

        db_tags.push(tag.clone());
        let new_tags = serde_json::to_string(&db_tags).unwrap();

        Self::update_data(pool, new_tags, "antitags".to_string(), user_id, service).await;

        Ok(())
    }

    pub async fn rm_antitag(
        pool: &Pool<Sqlite>,
        tag: String,
        user_id: i64,
        service: &Services,
    ) -> Result<(), String> {
        if !Self::exist_user(pool, user_id, service).await.unwrap() {
            return Err("User not exist".to_string());
        }

        let user_results = Self::fetch_all_user(pool, service).await;
        let user = user_results.iter().find(|x| x.user_id == user_id).unwrap();

        let mut db_tags = serde_json::from_str::<Vec<String>>(&user.antitags).unwrap_or_default();

        if !db_tags.iter().any(|i| *i == tag) {
            return Err("Tag not found".to_string());
        }

        db_tags.retain(|x| *x != tag);
        let new_tags = serde_json::to_string(&db_tags).unwrap();

        Self::update_data(pool, new_tags, "antitags".to_string(), user_id, service).await;

        Ok(())
    }

    pub async fn add_history(
        pool: &Pool<Sqlite>,
        user_id: i64,
        history: Vec<String>,
        service: &Services,
    ) -> std::io::Result<()> {
        let user_results = Self::fetch_all_user(pool, service).await;
        let user = user_results.iter().find(|x| x.user_id == user_id).unwrap();

        let mut db_history = serde_json::from_str::<Vec<String>>(&user.history).unwrap_or_default();

        Utils::remove_dub(&mut db_history, &history);

        let hist = [db_history, history].concat();
        let new_history = serde_json::to_string(&hist).unwrap();

        Self::update_data(pool, new_history, "history".to_string(), user_id, service).await;

        Ok(())
    }
}
