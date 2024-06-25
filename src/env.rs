pub fn env_opt(name: &str) -> Option<String> {
  std::env::var_os(name).and_then(|v| v.into_string().ok())
}

pub fn env_opt_present(name: &str) -> Option<String> {
  env_opt(name).filter(|v| !v.is_empty())
}
