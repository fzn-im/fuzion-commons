use std::time::Duration;

use awc::error::SendRequestError;
use thiserror::Error;
use tokio::time::sleep;

use crate::http::create_http_client;

pub struct Healthcheck {
  pub url: String,
}

impl Healthcheck {
  pub async fn run(&self) -> bool {
    for i in 1..=3 {
      println!("Healthcheck #{i}...");

      if test(&self.url).await {
        break;
      }

      if i == 3 {
        println!("Giving up...");

        return false;
      }

      sleep(Duration::from_secs(15)).await;
    }

    false
  }
}

async fn test(check_url: &str) -> bool {
  println!("Healthcheck endpoint: {check_url}");

  if (create_http_client().get(check_url).send().await).is_err() {
    println!("Healthcheck failed.");

    return false;
  }

  println!("Healthcheck successful.");

  true
}

#[derive(Debug, Error)]
pub enum HealthcheckError {
  #[error(transparent)]
  Send(#[from] SendRequestError),
}
