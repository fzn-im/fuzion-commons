use actix_web::http::Uri;
use deadpool_postgres::{Manager, ManagerConfig, Pool as Deadpool, RecyclingMethod};

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

#[derive(Clone, Debug, Deserialize)]
pub struct LoggingConfig {
  #[serde(default = "default_true")]
  pub log_to_stdout: bool,
  pub log_file: Option<String>,
  #[serde(deserialize_with = "deserialize_log_level")]
  pub log_level: slog::Level,
}

impl Default for LoggingConfig {
  fn default() -> Self {
    Self {
      log_to_stdout: true,
      log_level: slog::Level::Debug,
      log_file: None,
    }
  }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
  pub host: String,
  pub port: u16,
  pub user: String,
  pub password: String,
  pub name: String,
}

impl Default for DatabaseConfig {
  fn default() -> Self {
    Self {
      host: "".to_owned(),
      port: 5678u16,
      name: "".to_owned(),
      user: "".to_owned(),
      password: "".to_owned(),
    }
  }
}

impl DatabaseConfig {
  pub async fn get_db_pool(&self) -> Result<PgPool, ()> {
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

    Deadpool::builder(manager).build().map_err(|_| ())
  }
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpConfigWithPublic {
  pub host: String,
  pub port: u16,
  pub secure: bool,
  pub public: HttpConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpConfigWithPublicPrivate {
  pub host: String,
  pub port: u16,
  pub secure: bool,
  pub private: HttpConfig,
  pub public: HttpConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpConfig {
  pub host: String,
  pub port: u16,
  pub secure: bool,
}

impl Default for HttpConfig {
  fn default() -> Self {
    Self {
      host: "".to_owned(),
      port: 80u16,
      secure: false,
    }
  }
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

impl Default for HttpConfigWithPublic {
  fn default() -> Self {
    Self {
      host: "".to_owned(),
      port: 80u16,
      secure: false,
      public: Default::default(),
    }
  }
}

impl Default for HttpConfigWithPublicPrivate {
  fn default() -> Self {
    Self {
      host: "".to_owned(),
      port: 80u16,
      secure: false,
      public: Default::default(),
      private: Default::default(),
    }
  }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct HttpEndpointConfig {
  pub endpoint: String,
}
