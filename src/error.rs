use std::collections::HashMap;

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::ser::Serialize;

#[derive(Debug)]
pub struct UnitError;

impl std::fmt::Display for UnitError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "UnitError")
  }
}

impl ResponseError for UnitError {
  fn status_code(&self) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
  }

  fn error_response(&self) -> HttpResponse {
    HttpResponse::InternalServerError().finish()
  }
}

#[derive(Debug, Serialize)]
pub struct ErrorMap {
  errors: HashMap<String, serde_json::Value>,
}

impl ErrorMap {
  pub fn new() -> Self {
    Self {
      errors: Default::default(),
    }
  }

  pub fn add_error<S, T>(&mut self, field: S, value: T)
  where
    S: Into<String>,
    T: Serialize,
  {
    self
      .errors
      .insert(field.into(), serde_json::to_value(&value).unwrap());
  }

  pub fn to_map(self) -> HashMap<String, serde_json::Value> {
    self.errors
  }

  pub fn len(&self) -> usize {
    self.errors.len()
  }
}
