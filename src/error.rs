use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};

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
