use actix_web::http::Uri;
use deadpool::managed::BuildError;
use deadpool_postgres::{Manager, ManagerConfig, Pool as Deadpool, RecyclingMethod};
use smart_default::SmartDefault;
use thiserror::Error;

use crate::db::PgPool;
use crate::serde::{default_true, deserialize_log_level};

pub fn clap_arg_to_log_level(level: &str) -> Result<slog::Level, String> {
  match level {
    "critical" => Ok(slog::Level::Critical),
    "debug" => Ok(slog::Level::Debug),
    "error" => Ok(slog::Level::Error),
    "trace" => Ok(slog::Level::Trace),
    "warning" => Ok(slog::Level::Warning),
    "info" => Ok(slog::Level::Info),
    _ => Err(String::from("Failed to parse log level.")),
  }
}

#[derive(Clone, Debug, Deserialize, SmartDefault)]
pub struct LoggingConfig {
  #[default = true]
  #[serde(default = "default_true")]
  pub log_to_stdout: bool,
  pub log_file: Option<String>,
  #[default(_code = "slog::Level::Info")]
  #[serde(deserialize_with = "deserialize_log_level")]
  pub log_level: slog::Level,
}

#[derive(Clone, Debug, Deserialize, SmartDefault)]
pub struct DatabaseConfig {
  #[default = "localhost"]
  pub host: String,
  #[default = 5432]
  pub port: u16,
  #[default = ""]
  pub user: String,
  #[default = ""]
  pub password: String,
  #[default = "fuzion-veritas"]
  pub name: String,
}

impl DatabaseConfig {
  pub async fn get_db_pool(&self) -> Result<PgPool, DatabaseConfigError> {
    let mut pg_config = tokio_postgres::Config::new();
    pg_config.user(&self.user);
    pg_config.password(&self.password);
    pg_config.dbname(&self.name);
    pg_config.host(&self.host);
    pg_config.port(self.port);

    let manager = Manager::from_config(
      pg_config,
      tokio_postgres::NoTls,
      ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
      },
    );

    Ok(Deadpool::builder(manager).build()?)
  }
}

#[derive(Clone, Debug, Error)]
pub enum DatabaseConfigError {
  #[error(transparent)]
  DeadpoolBuildError(#[from] BuildError),
}

#[derive(Clone, Debug, Deserialize, SmartDefault)]
pub struct HttpConfigWithPublic {
  #[default = "localhost"]
  pub host: String,
  #[default = 5432]
  pub port: u16,
  #[default = false]
  #[serde(default)]
  pub secure: bool,
  pub public: HttpConfig,
}

#[derive(Clone, Debug, Deserialize, SmartDefault)]
pub struct HttpConfigWithPublicPrivate {
  #[default = ""]
  pub host: String,
  pub port: u16,
  #[default = false]
  #[serde(default)]
  pub secure: bool,
  pub private: HttpConfig,
  pub public: HttpConfig,
}

#[derive(Clone, Debug, Deserialize, SmartDefault)]
pub struct HttpConfig {
  #[default = ""]
  pub host: String,
  #[default = 80]
  pub port: u16,
  #[default = false]
  #[serde(default)]
  pub secure: bool,
}

impl HttpConfig {
  pub fn get_uri(&self) -> Uri {
    let mut public_path = String::new();

    public_path += match self.secure {
      true => "https",
      _ => "http",
    };

    public_path += &format!("://{}", &self.host);

    if !(!self.secure && self.port == 80) && !(self.secure && self.port == 443) {
      public_path += &format!(":{}", &self.port);
    }

    public_path.parse().expect("Invalid URI provided")
  }

  pub fn get_socket_addr(&self) -> (String, u16) {
    (self.host.to_owned(), self.port)
  }
}

impl HttpConfigWithPublic {
  pub fn get_uri(&self) -> Uri {
    let mut public_path = String::new();

    public_path += match self.secure {
      true => "https",
      _ => "http",
    };

    public_path += &format!("://{}", &self.host);

    if !(!self.secure && self.port == 80) && !(self.secure && self.port == 443) {
      public_path += &format!(":{}", &self.port);
    }

    public_path.parse().expect("Invalid URI provided")
  }

  pub fn get_public_uri(&self) -> Uri {
    self.public.get_uri()
  }

  pub fn get_socket_addr(&self) -> (String, u16) {
    (self.host.to_owned(), self.port)
  }
}

impl HttpConfigWithPublicPrivate {
  pub fn get_uri(&self) -> Uri {
    let mut public_path = String::new();

    public_path += match self.secure {
      true => "https",
      _ => "http",
    };

    public_path += &format!("://{}", &self.host);

    if !(!self.secure && self.port == 80) && !(self.secure && self.port == 443) {
      public_path += &format!(":{}", &self.port);
    }

    public_path.parse().expect("Invalid URI provided")
  }

  pub fn get_public_uri(&self) -> Uri {
    self.public.get_uri()
  }

  pub fn get_private_uri(&self) -> Uri {
    self.private.get_uri()
  }

  pub fn get_socket_addr(&self) -> (String, u16) {
    (self.host.to_owned(), self.port)
  }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct HttpEndpointConfig {
  pub endpoint: String,
}
