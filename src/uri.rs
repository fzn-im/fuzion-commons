use std::path::Path;

use http::{uri, Uri};

pub trait UriUtils {
  fn path_join(&self, add: &str) -> Uri;
}

impl UriUtils for Uri {
  fn path_join(&self, add: &str) -> Uri {
    let this = self.to_owned();

    let path = this.path().to_owned();
    let query = this.query().map(|v| v.to_owned());

    let builder = uri::Builder::from(this).path_and_query(format!(
      "{}{}",
      Path::new(&path).join(add).to_string_lossy(),
      query
        .as_ref()
        .map(|v| format!("?{}", v.as_str()))
        .unwrap_or(String::from("")),
    ));

    builder.build().unwrap()
  }
}

pub fn path_join() {}
