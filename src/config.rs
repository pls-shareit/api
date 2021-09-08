//! Manages configuration of the server and Rocket.
use byte_unit::Byte;
use rocket::config::{Environment, Limits};
use rocket::http::Status;
use rocket::response::status;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::Duration;
use std::{env, process};

pub const DEFAULT_PASSWORD: &str = "password";

fn default_highlighting_language() -> String { "auto".into() }
fn default_mime_type() -> String { "application/octet-stream".into() }
fn default_expiry_check_interval() -> Duration { Duration::from_secs(60) }
fn default_min_name_length() -> u8 { 1 }
fn default_max_name_length() -> u8 { 32 }
fn default_random_name_length() -> RwLock<usize> { RwLock::new(8) }
fn default_random_name_attempt_limit() -> u8 { 3 }
fn default_max_upload_size() -> Byte { Byte::from_str("2 MB").unwrap() }
fn default_max_link_length() -> u16 { 255 }
fn default_allowed_link_schemes() -> Vec<String> { vec!["http".into(), "https".into()] }
fn default_disallowed_mime_types() -> Vec<String> { vec!["text/html".into()] }
fn default_bind_address() -> String { "127.0.0.1".into() }
fn default_bind_port() -> u16 { 8000 }
fn default_db_host() -> String { "127.0.0.1".into() }
fn default_db_port() -> u16 { 5432 }
fn default_db_user() -> String { "shareit".into() }
fn default_db_name() -> String { "shareit".into() }
fn default_upload_dir() -> PathBuf { "/var/shareit/shares/".into() }

fn default_passwords() -> HashMap<String, Vec<Permission>> {
    HashMap::from([(
        DEFAULT_PASSWORD.into(),
        vec![
            Permission::CreateAny,
            Permission::UpdateOwn,
            Permission::CustomName,
        ],
    )])
}

fn default_highlighting_languages() -> Vec<String> {
    include_str!("res/languages.txt")
        .split('\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub frontend_path: Option<PathBuf>,
    #[serde(default = "default_upload_dir")]
    pub upload_dir: PathBuf,
    #[serde(default = "default_highlighting_languages")]
    pub highlighting_languages: Vec<String>,
    #[serde(default = "default_highlighting_language")]
    pub default_highlighting_language: String,
    #[serde(default = "default_mime_type")]
    pub default_mime_type: String,
    #[serde(with = "humantime_serde", default = "default_expiry_check_interval")]
    pub expiry_check_interval: Duration,
    #[serde(default = "default_passwords")]
    pub passwords: HashMap<String, Vec<Permission>>,
    #[serde(default)]
    pub names: NamesConfig,
    #[serde(default)]
    pub restrictions: RestrictionsConfig,
    pub network: NetworkConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    CreateAny,
    CreateLink,
    CreateFile,
    CreatePaste,
    UpdateOwn,
    UpdateAny,
    CustomName,
}

#[derive(Deserialize)]
pub struct NamesConfig {
    #[serde(default = "default_min_name_length")]
    pub min_length: u8,
    #[serde(default = "default_max_name_length")]
    pub max_length: u8,
    #[serde(default = "default_random_name_length")]
    random_length: RwLock<usize>,
    #[serde(default = "default_random_name_attempt_limit")]
    pub random_attempt_limit: u8,
}

impl NamesConfig {
    pub fn get_random_length(&self) -> Result<usize, status::Custom<String>> {
        self.random_length
            .read()
            .map_err(|_| {
                status::Custom(Status::InternalServerError, "Internal server error.".into())
            })
            .map(|value| *value)
    }

    pub fn incr_random_length(&self) -> Result<(), status::Custom<String>> {
        let mut value = self.random_length.write().map_err(|_| {
            status::Custom(Status::InternalServerError, "Internal server error.".into())
        })?;
        *value += 1;
        Ok(())
    }
}

impl Default for NamesConfig {
    fn default() -> NamesConfig {
        NamesConfig {
            min_length: default_min_name_length(),
            max_length: default_max_name_length(),
            random_length: default_random_name_length(),
            random_attempt_limit: default_random_name_attempt_limit(),
        }
    }
}

#[derive(Deserialize)]
pub struct RestrictionsConfig {
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size: Byte,
    // Note: Link length cannot be more than either max upload size or 2047.
    #[serde(default = "default_max_link_length")]
    pub max_link_length: u16,
    #[serde(with = "humantime_serde", default)]
    pub max_expiry_time: Option<Duration>,
    #[serde(default)]
    pub allowed_mime_types: Vec<String>,
    #[serde(default = "default_disallowed_mime_types")]
    pub disallowed_mime_types: Vec<String>,
    #[serde(default = "default_allowed_link_schemes")]
    pub allowed_link_schemes: Vec<String>,
}

impl Default for RestrictionsConfig {
    fn default() -> Self {
        RestrictionsConfig {
            max_upload_size: default_max_upload_size(),
            max_link_length: default_max_link_length(),
            max_expiry_time: None,
            allowed_mime_types: vec![],
            disallowed_mime_types: vec![],
            allowed_link_schemes: default_allowed_link_schemes(),
        }
    }
}

#[derive(Deserialize)]
pub struct NetworkConfig {
    pub host: url::Url,
    #[serde(default = "default_bind_address")]
    pub address: String,
    #[serde(default = "default_bind_port")]
    pub port: u16,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub pass: String,
    #[serde(default = "default_db_host")]
    pub host: String,
    #[serde(default = "default_db_port")]
    pub port: u16,
    #[serde(default = "default_db_user")]
    pub user: String,
    #[serde(default = "default_db_name")]
    pub name: String,
}

impl Config {
    pub fn load() -> Self {
        let args: Vec<OsString> = env::args_os().collect();
        if args.len() < 2 {
            eprintln!("Usage: {} <config file>", args[0].to_string_lossy());
            process::exit(1);
        }
        let mut file =
            File::open(args[1].clone()).expect("Could not open config file (shareit.toml)");
        let mut raw = String::new();
        file.read_to_string(&mut raw)
            .expect("Could not read or decode config file");
        let config: Config = toml::from_str(&raw).expect("Could not parse config file");
        create_dir_all(&config.upload_dir).expect("Could not create upload directory");
        config
    }

    pub fn configure_rocket(&self) -> rocket::config::Config {
        let mut databases: HashMap<String, _> = HashMap::new();
        let mut database: HashMap<String, _> = HashMap::new();
        database.insert("url".into(), self.make_database_url());
        databases.insert("database".into(), database);
        rocket::config::Config::build(Environment::Production)
            .extra("databases", databases)
            .address(self.network.address.clone())
            .port(self.network.port)
            .limits(self.make_rocket_limits())
            .finalize()
            .expect("Could not configure Rocket")
    }

    pub fn make_database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.user,
            self.database.pass,
            self.database.host,
            self.database.port,
            self.database.name
        )
    }

    fn make_rocket_limits(&self) -> Limits {
        let limit = self.restrictions.max_upload_size.get_bytes();
        Limits::new()
            .limit("form", limit)
            .limit("data-form", limit)
            .limit("file", limit)
            .limit("string", limit)
            .limit("bytes", limit)
    }
}
