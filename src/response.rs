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
    let status = self.status();
    if status.is_success() {
      return Ok(self);
    }

    let body = match self.body().await {
      Ok(body) => body,
      Err(err) => {
        return Err(ResponseHandlingError::ErrorStatusBody(
          status,
          format!("<failed to read response body: {err}>"),
        ));
      }
    };

    let body_text = String::from_utf8_lossy(&body).into_owned();

    if let Ok(response) = serde_json::from_slice::<serde_json::Value>(&body) {
      return Err(ResponseHandlingError::ErrorResponse(status, response));
    }

    Err(ResponseHandlingError::ErrorStatusBody(status, body_text))
  }
}

#[derive(Debug, Error, ResponseError)]
pub enum ResponseHandlingError {
  #[error("HTTP {0} error response body: {1}")]
  ErrorResponse(StatusCode, serde_json::Value),
  #[error("HTTP error status {0}")]
  ErrorStatus(StatusCode),
  #[error("HTTP error status {0}: {1}")]
  ErrorStatusBody(StatusCode, String),
  #[error("HTTP response JSON parse error: {0}")]
  JsonPayloadError(#[from] JsonPayloadError),
}
