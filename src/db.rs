use std::future::Future;
use std::pin::Pin;

use actix_web::*;
use actix_web_thiserror::ResponseError;
use futures::TryFutureExt;
use thiserror::Error;

use crate::error::UnitError;

pub type PgPool = deadpool_postgres::Pool;

pub struct PgClient<'a> {
  inner: PgClientInner<'a>,
}

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
    PgClient::from_client(from)
  }
}

impl<'a> From<deadpool_postgres::Transaction<'a>> for PgClient<'a> {
  fn from(from: deadpool_postgres::Transaction<'a>) -> Self {
    PgClient::from_transaction(from)
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

#[derive(Debug, Error, ResponseError)]
pub enum PgClientError {
  #[error("internal_error")]
  InternalError,
  #[error("tokio_postgres error: {0}")]
  PostgresError(#[from] tokio_postgres::Error),
  #[error("deadpool_pool error: {0}")]
  DeadpoolPoolError(#[from] DeadpoolPoolError),
}

pub type DeadpoolPoolError = deadpool::managed::PoolError<tokio_postgres::Error>;
