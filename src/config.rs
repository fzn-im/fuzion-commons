use deadpool_postgres::{Manager, ManagerConfig, Pool as Deadpool, RecyclingMethod};

use crate::db::PgPool;
use crate::serde::{default_true, deserialize_log_level};

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
