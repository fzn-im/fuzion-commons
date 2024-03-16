use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::iter::FromIterator;
use std::pin::Pin;
use std::rc::Rc;
use std::slice::Iter;

use actix_web::*;
use futures::TryFutureExt;
use thiserror::Error;
use tokio_postgres::row::RowIndex;
use tokio_postgres::types::FromSql;
use tokio_postgres::Row;

use crate::domain::error::UnitError;

pub type PgPool = deadpool_postgres::Pool;

pub struct PgClient<'a> {
  inner: PgClientInner<'a>,
}

#[allow(dead_code)]
impl<'a> PgClient<'a> {
  pub fn from_client(client: deadpool_postgres::Client) -> PgClient<'a> {
    client.into()
  }

  fn from_transaction(transaction: deadpool_postgres::Transaction<'a>) -> PgClient<'a> {
    transaction.into()
  }

  pub async fn from_pool(pool: &PgPool) -> Result<PgClient<'a>, PgClientError> {
    Ok(pool.get().await.map(|client| Self::from_client(client))?)
  }
}

#[allow(dead_code)]
impl<'a> PgClient<'a> {
  pub async fn prepare(&self, query: &str) -> Result<tokio_postgres::Statement, PgClientError> {
    let stmt = match &self.inner {
      PgClientInner::Client(client) => client.prepare_cached(query).await,
      PgClientInner::Transaction(transaction) => transaction.prepare_cached(query).await,
      _ => Err(PgClientError::InternalError)?,
    }
    .map_err(|err| {
      error!("Failed to prepare query: {} <{}>", err, query);
      err
    })?;

    Ok(stmt)
  }

  pub async fn prepare_cached(
    &self,
    query: &str,
  ) -> Result<tokio_postgres::Statement, PgClientError> {
    let stmt = match &self.inner {
      PgClientInner::Client(client) => client.prepare_cached(query).await,
      PgClientInner::Transaction(transaction) => transaction.prepare_cached(query).await,
      _ => Err(PgClientError::InternalError)?,
    }
    .map_err(|err| {
      error!("Failed to prepare query: {} <{}>", err, query);
      err
    })?;

    Ok(stmt)
  }

  pub async fn query<T>(
    &self,
    query: &T,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
  ) -> Result<Vec<tokio_postgres::Row>, PgClientError>
  where
    T: tokio_postgres::ToStatement,
  {
    Ok(match &self.inner {
      PgClientInner::Client(client) => client.query(query, params).await?,
      PgClientInner::Transaction(transaction) => transaction.query(query, params).await?,
      _ => Err(PgClientError::InternalError)?,
    })
  }

  pub async fn execute<T>(
    &self,
    query: &T,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
  ) -> Result<u64, PgClientError>
  where
    T: ?Sized + tokio_postgres::ToStatement + Sync + Send,
  {
    Ok(match &self.inner {
      PgClientInner::Client(client) => client.execute(query, params).await?,
      PgClientInner::Transaction(transaction) => transaction.execute(query, params).await?,
      _ => Err(PgClientError::InternalError)?,
    })
  }

  pub async fn simple_query(
    &self,
    query: &str,
  ) -> Result<Vec<tokio_postgres::SimpleQueryMessage>, PgClientError> {
    Ok(match &self.inner {
      PgClientInner::Client(client) => client.simple_query(query).await?,
      PgClientInner::Transaction(transaction) => transaction.simple_query(query).await?,
      _ => Err(PgClientError::InternalError)?,
    })
  }

  pub async fn transaction(&mut self) -> Result<PgClient<'_>, PgClientError> {
    Ok(match &mut self.inner {
      PgClientInner::Client(client) => client.transaction().await?.into(),
      PgClientInner::Transaction(transaction) => transaction.transaction().await?.into(),
      _ => Err(PgClientError::InternalError)?,
    })
  }

  pub async fn commit(mut self) -> Result<(), PgClientError> {
    let mut _self = PgClientInner::Empty;
    std::mem::swap(&mut self.inner, &mut _self);

    Ok(match _self {
      PgClientInner::Transaction(transaction) => transaction.commit().await,
      _ => Ok(()),
    }?)
  }

  pub async fn rollback(mut self) -> Result<(), PgClientError> {
    let mut _self = PgClientInner::Empty;
    std::mem::swap(&mut self.inner, &mut _self);

    Ok(match _self {
      PgClientInner::Transaction(transaction) => transaction.rollback().await,
      _ => Ok(()),
    }?)
  }
}

impl<'a> From<deadpool_postgres::Client> for PgClient<'a> {
  fn from(from: deadpool_postgres::Client) -> Self {
    PgClient {
      inner: PgClientInner::Client(from),
    }
  }
}

impl<'a> From<deadpool_postgres::Transaction<'a>> for PgClient<'a> {
  fn from(from: deadpool_postgres::Transaction<'a>) -> Self {
    PgClient {
      inner: PgClientInner::Transaction(from),
    }
  }
}

pub enum PgClientInner<'a> {
  Client(deadpool_postgres::Client),
  Transaction(deadpool_postgres::Transaction<'a>),
  Empty,
}

impl<'a> FromRequest for PgClient<'a> {
  type Error = UnitError;
  type Future = Pin<Box<dyn Future<Output = Result<PgClient<'a>, Self::Error>> + 'static>>;

  fn from_request(http: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
    let pool = match http
      .app_data::<web::Data<PgPool>>()
      .map(|pool| pool.as_ref().to_owned())
    {
      Some(pool) => pool,
      _ => {
        error!("No Pool found during FromRequest.");
        return Box::pin(async move { Err(UnitError) });
      }
    };

    Box::pin(async move {
      pool
        .get()
        .map_ok(|conn| PgClient::from_client(conn))
        .map_err(|_| UnitError)
        .await
    })
  }
}

pub struct PgRow<'a> {
  row: &'a Row,
  columns: Rc<HashMap<&'a str, usize>>,
}

impl<'a> PgRow<'a> {
  pub fn len(&self) -> usize {
    self.row.len()
  }

  pub fn is_empty(&self) -> bool {
    self.row.is_empty()
  }

  pub fn get<I, T>(&'a self, idx: I) -> T
  where
    I: RowIndex + Display,
    T: FromSql<'a>,
  {
    self.row.get::<I, T>(idx)
  }

  pub fn get_named<T>(&'a self, name: &str) -> T
  where
    T: FromSql<'a>,
  {
    self.row.get::<_, T>(self.columns.get(name).unwrap())
  }

  pub fn get_named_json<T>(&'a self, name: &str) -> T
  where
    T: for<'b> serde::de::Deserialize<'b>,
  {
    let value = self
      .row
      .get::<_, serde_json::Value>(self.columns.get(name).unwrap());
    serde_json::from_value(value).unwrap()
  }

  pub fn get_opt_named_json<T>(&'a self, name: &str) -> Option<T>
  where
    T: for<'b> serde::de::Deserialize<'b>,
  {
    let value = self
      .row
      .get::<_, Option<serde_json::Value>>(self.columns.get(name).unwrap());
    value.map(|value| serde_json::from_value(value).unwrap())
  }

  pub fn from_vec(mut rows: Vec<&'a Row>) -> Vec<PgRow<'a>> {
    let columns = Rc::new(
      rows
        .get(0)
        .map(|row| {
          HashMap::from_iter(
            row
              .columns()
              .iter()
              .enumerate()
              .map(|(i, column)| (column.name(), i)),
          )
        })
        .unwrap_or_else(|| HashMap::new()),
    );

    rows
      .drain(..)
      .map(|row| PgRow {
        row,
        columns: columns.clone(),
      })
      .collect::<Vec<PgRow<'a>>>()
  }

  pub fn from_iter(iter: &mut Iter<'a, Row>) -> Vec<PgRow<'a>> {
    let mut rows = vec![];
    let mut columns = None;

    while let Some(row) = iter.next() {
      if let None = columns {
        columns = Some(Rc::new(HashMap::from_iter(
          row
            .columns()
            .iter()
            .enumerate()
            .map(|(i, column)| (column.name(), i)),
        )));
      }

      if let Some(ref columns) = columns {
        rows.push(PgRow {
          row,
          columns: columns.clone(),
        });
      }
    }

    rows
  }
}

impl<'a> From<&'a Row> for PgRow<'a> {
  fn from(row: &'a Row) -> Self {
    let columns = Rc::new(HashMap::from_iter(
      row
        .columns()
        .iter()
        .enumerate()
        .map(|(i, column)| (column.name(), i)),
    ));

    Self { row, columns }
  }
}

#[derive(Debug, Error)]
pub enum PgClientError {
  #[error("internal error")]
  InternalError,
  #[error("tokio_postgres error: {0}")]
  PostgresError(tokio_postgres::Error),
  #[error("deadpool pool error: {0}")]
  DeadpoolPoolError(DeadpoolPoolError),
}

pub type DeadpoolPoolError = deadpool::managed::PoolError<tokio_postgres::Error>;

impl From<tokio_postgres::Error> for PgClientError {
  fn from(from: tokio_postgres::Error) -> Self {
    PgClientError::PostgresError(from)
  }
}

impl From<DeadpoolPoolError> for PgClientError {
  fn from(from: DeadpoolPoolError) -> Self {
    PgClientError::DeadpoolPoolError(from)
  }
}

impl From<PgClientError> for Error {
  fn from(from: PgClientError) -> Error {
    HttpPostgresError(from)
  }
}

#[allow(non_snake_case)]
pub fn HttpPostgresError<T: Into<PgClientError> + Display>(err: T) -> actix_web::error::Error {
  error!("Postgres error: {}.", &err);
  actix_web::error::ErrorInternalServerError("")
}

pub struct ToSqlIterable {
  pub values: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + 'static>>,
}

impl ToSqlIterable {
  pub fn new() -> ToSqlIterable {
    ToSqlIterable { values: Vec::new() }
  }

  pub fn push<T: tokio_postgres::types::ToSql + Sync + 'static>(&mut self, value: T) {
    self.values.push(Box::new(value));
  }

  pub fn len(&self) -> usize {
    self.values.len()
  }

  pub fn to_vec(&self) -> Vec<&(dyn tokio_postgres::types::ToSql + Sync)> {
    self
      .values
      .iter()
      .map(|value| &**value as &(dyn tokio_postgres::types::ToSql + Sync))
      .collect()
  }
}
