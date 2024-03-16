use regex::Regex;
use serde::{de, Deserializer};

pub fn deserialize_log_level<'de, D>(de: D) -> Result<slog::Level, D::Error>
where
  D: Deserializer<'de>,
{
  let level: String = de::Deserialize::deserialize(de)?;
  str_to_log_level(Some(&level)).map_err(|_| de::Error::custom("Invalid loglevel"))
}

pub fn str_to_log_level(level: Option<&str>) -> Result<slog::Level, ()> {
  match level {
    Some("critical") => Ok(slog::Level::Critical),
    Some("debug") => Ok(slog::Level::Debug),
    Some("error") => Ok(slog::Level::Error),
    Some("trace") => Ok(slog::Level::Trace),
    Some("warning") => Ok(slog::Level::Warning),
    Some("info") | None => Ok(slog::Level::Info),
    Some(_) => Err(()),
  }
}

pub fn deserialize_regex<'de, D>(de: D) -> Result<Regex, D::Error>
where
  D: Deserializer<'de>,
{
  let regex: String = de::Deserialize::deserialize(de)?;
  Regex::new(&regex).map_err(de::Error::custom)
}

pub fn default_true() -> bool {
  true
}

pub fn default_false() -> bool {
  false
}
