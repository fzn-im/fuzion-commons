use actix_web_thiserror::ResponseError;
use async_trait::async_trait;
use awc::error::{JsonPayloadError, PayloadError};
use awc::ClientResponse;
use bytes::Bytes;
use futures::Stream;
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
      let response: ErrorResponse = self.json().await?;

      return Err(ResponseHandlingError::ErrorResponse(response));
    }

    Ok(self)
  }
}

#[derive(Debug, Error, ResponseError)]
pub enum ResponseHandlingError {
  #[error("Error response")]
  ErrorResponse(ErrorResponse),
  #[error(transparent)]
  JsonPayloadError(#[from] JsonPayloadError),
}
