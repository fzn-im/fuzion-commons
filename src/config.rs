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
