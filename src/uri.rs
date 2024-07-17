use http::Uri;
use url::Url;

pub trait UriUtils {
  fn path_join(&self, add: &str) -> String;
}

impl UriUtils for Uri {
  fn path_join(&self, add: &str) -> String {
    self.to_string().path_join(add)
  }
}

impl UriUtils for &str {
  fn path_join(&self, add: &str) -> String {
    let mut url = Url::parse(self).unwrap();

    if let Ok(mut path) = url.path_segments_mut() {
      path.extend(add.split("/"));
    }

    url.to_string()
  }
}

impl UriUtils for String {
  fn path_join(&self, add: &str) -> String {
    let mut url = Url::parse(self).unwrap();

    if let Ok(mut path) = url.path_segments_mut() {
      path.extend(add.split("/"));
    }

    url.to_string()
  }
}

pub fn path_join() {}
