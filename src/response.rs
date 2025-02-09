use actix_http::StatusCode;
use actix_web_thiserror::ResponseError;
use async_trait::async_trait;
use awc::error::{JsonPayloadError, PayloadError};
use awc::ClientResponse;
use bytes::Bytes;
use futures::Stream;
use thiserror::Error;

#[async_trait(?Send)]
pub trait ResponseHandling: Sized {
  async fn handle_error(mut self) -> Result<Self, ResponseHandlingError> {
    Ok(self)
  }
}

#[async_trait(?Send)]
impl<S> ResponseHandling for ClientResponse<S>
where
  S: Stream<Item = Result<Bytes, PayloadError>>,
{
  async fn handle_error(mut self) -> Result<Self, ResponseHandlingError> {
    if self.status() != 200 {
      match self.json().await {
        Ok(response) => return Err(ResponseHandlingError::ErrorResponse(response)),
        Err(_) => {
          return match self.body().await {
            Ok(body) => Err(ResponseHandlingError::ErrorStatusBody(
              self.status(),
              String::from_utf8_lossy(&body[..]).to_string(),
            )),
            _ => Err(ResponseHandlingError::ErrorStatus(self.status())),
          }
        }
      }
    }

    Ok(self)
  }
}

#[derive(Debug, Error, ResponseError)]
pub enum ResponseHandlingError {
  #[error("Error response")]
  ErrorResponse(serde_json::Value),
  #[error("Response status error: {0}")]
  ErrorStatus(StatusCode),
  #[error("Response status error: {0} {1}")]
  ErrorStatusBody(StatusCode, String),
  #[error(transparent)]
  JsonPayloadError(#[from] JsonPayloadError),
}
