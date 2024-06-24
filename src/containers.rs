use std::env;
use std::path::Path;

pub fn in_container() -> bool {
  Path::new("/.dockerenv").exists()
    || env::var("KUBERNETES_SERVICE_HOST")
      .map(|val| val.len() > 0)
      .unwrap_or(false)
}

pub fn if_container<T>(_if: T, _else: T) -> T {
  if in_container() {
    _if
  } else {
    _else
  }
}
