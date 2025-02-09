use std::collections::HashMap;

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use actix_web_thiserror::ResponseTransform;
use serde::ser::Serialize;

#[derive(Default)]
pub struct ErrorResponseTransform;

impl ResponseTransform for ErrorResponseTransform {
  fn transform(
    &self,
    _name: &str,
    _err: &dyn std::error::Error,
    status_code: actix_web::http::StatusCode,
    reason: Option<serde_json::Value>,
    _type: Option<String>,
    details: Option<serde_json::Value>,
  ) -> HttpResponse {
    if let Some(reason) = reason {
      let mut response: HashMap<String, serde_json::Value> = HashMap::new();
      response.insert(String::from("error"), reason);

      if let Some(details) = details {
        response.insert(String::from("details"), details);
      }

      if let Some(_type) = _type {
        response.insert(String::from("type"), _type.into());
      }

      HttpResponse::build(status_code).json(response)
    } else {
      HttpResponse::InternalServerError().finish()
    }
  }

  fn default_error_status_code(&self) -> actix_web::http::StatusCode {
    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {}

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

#[derive(Debug, Default, Serialize)]
pub struct ErrorMap(HashMap<String, serde_json::Value>);

impl ErrorMap {
  pub fn add_error<S, T>(&mut self, field: S, value: T)
  where
    S: Into<String>,
    T: Serialize,
  {
    self
      .0
      .insert(field.into(), serde_json::to_value(&value).unwrap());
  }

  pub fn to_map(self) -> HashMap<String, serde_json::Value> {
    self.0
  }

  pub fn len(&self) -> usize {
    self.0.values().fold(0, |mut acc, value| {
      if let serde_json::Value::Array(value) = value {
        acc += value.len();
      }

      acc
    })
  }

  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}
