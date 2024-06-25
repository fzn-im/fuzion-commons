pub fn env_opt(name: &str) -> Option<String> {
  std::env::var_os(name).and_then(|v| v.into_string().ok())
}
