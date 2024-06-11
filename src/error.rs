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
  error: String,
  errors: HashMap<String, serde_json::Value>,
  _type: String,
}

impl ErrorMap {
  pub fn new<T: Into<String>>(error: T) -> Self {
    Self {
      error: error.into(),
      errors: Default::default(),
      _type: String::from("error_map"),
    }
  }

  pub fn add_error<S: Serialize>(&mut self, field: &str, value: S) {
    self
      .errors
      .insert(field.to_owned(), serde_json::to_value(&value).unwrap());
  }

  pub fn len(&self) -> usize {
    self.errors.len()
  }
}
