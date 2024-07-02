use std::str::FromStr;

pub fn env_opt<T>(name: &str) -> Option<T>
where
  T: FromStr,
{
  std::env::var_os(name)
    .and_then(|v| v.into_string().ok())
    .and_then(|v| v.parse().ok())
}

pub fn env_present<T>(name: &str) -> Option<T>
where
  T: FromStr,
{
  std::env::var_os(name)
    .and_then(|v| v.into_string().ok())
    .filter(|v| !v.is_empty())
    .and_then(|v| v.parse().ok())
}

pub fn env_present_or<T>(name: &str, _else: T) -> T
where
  T: FromStr,
{
  env_present(name).unwrap_or(_else)
}
