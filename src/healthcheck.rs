use std::time::Duration;

use thiserror::Error;
use tokio::time::sleep;

use crate::http::create_http_client;

pub struct Healthcheck {
  pub url: String,
}

impl Healthcheck {
  pub async fn run(&self) -> Result<(), HealthcheckError> {
    'running: loop {
      for i in 1..=3 {
        println!("Healthcheck #{i}...");

        if test(&self.url).await? {
          break;
        }

        if i == 3 {
          println!("Giving up...");

          break 'running;

          // process::exit(1);
        }

        sleep(Duration::from_secs(15)).await;
      }
    }

    Ok(())
  }
}

async fn test(check_url: &str) -> Result<bool, HealthcheckError> {
  println!("Healthcheck endpoint: {check_url}");

  if create_http_client().get(check_url).send().await.is_ok() {
    println!("Healthcheck successful.");

    return Ok(true);
  }

  println!("Healthcheck failed.");

  Ok(false)
}

#[derive(Debug, Error)]
pub enum HealthcheckError {}
