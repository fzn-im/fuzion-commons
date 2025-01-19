use regex::Regex;
use serde::{de, ser, Deserializer, Serializer};

pub fn deserialize_log_level<'de, D>(de: D) -> Result<slog::Level, D::Error>
where
  D: Deserializer<'de>,
{
  let level: String = de::Deserialize::deserialize(de)?;
  str_to_log_level(Some(&level)).ok_or(de::Error::custom("Invalid loglevel"))
}

pub fn serialize_log_level<S>(level: &slog::Level, s: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let value: String =
    log_level_to_str(Some(*level)).ok_or(ser::Error::custom("Invalid loglevel"))?;
  s.serialize_str(&value)
}

pub fn str_to_log_level(level: Option<&str>) -> Option<slog::Level> {
  match level {
    Some("critical") => Some(slog::Level::Critical),
    Some("debug") => Some(slog::Level::Debug),
    Some("error") => Some(slog::Level::Error),
    Some("trace") => Some(slog::Level::Trace),
    Some("warning") => Some(slog::Level::Warning),
    Some("info") | None => Some(slog::Level::Info),
    Some(_) => None,
  }
}

pub fn log_level_to_str(level: Option<slog::Level>) -> Option<String> {
  match level {
    Some(slog::Level::Critical) => Some(String::from("critical")),
    Some(slog::Level::Debug) => Some(String::from("debug")),
    Some(slog::Level::Error) => Some(String::from("error")),
    Some(slog::Level::Trace) => Some(String::from("trace")),
    Some(slog::Level::Warning) => Some(String::from("warning")),
    Some(_) | None => Some(String::from("info")),
  }
}

pub fn deserialize_regex<'de, D>(de: D) -> Result<Regex, D::Error>
where
  D: Deserializer<'de>,
{
  let regex: String = de::Deserialize::deserialize(de)?;
  Regex::new(&regex).map_err(de::Error::custom)
}

pub fn serialize_regex<S>(regex: &Regex, s: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  s.serialize_str(&regex.to_string())
}

pub fn default_true() -> bool {
  true
}

pub fn default_false() -> bool {
  false
}
