use actix_web_thiserror::ResponseError;
use async_trait::async_trait;
use awc::error::{JsonPayloadError, PayloadError};
use awc::ClientResponse;
use bytes::Bytes;
use futures::Stream;
use http::StatusCode;
use thiserror::Error;

use crate::error::ErrorResponse;

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
        Err(_) => return Err(ResponseHandlingError::ErrorStatus(self.status())),
      }
    }

    Ok(self)
  }
}

#[derive(Debug, Error, ResponseError)]
pub enum ResponseHandlingError {
  #[error("Error response")]
  ErrorResponse(ErrorResponse),
  #[error("Response status error: {0}")]
  ErrorStatus(StatusCode),
  #[error(transparent)]
  JsonPayloadError(#[from] JsonPayloadError),
}
