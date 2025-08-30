use std::backtrace::Backtrace;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use actix_web::*;
use actix_web_thiserror::ResponseError;
use futures::TryFutureExt;
use thiserror::Error;

#[derive(Clone)]
pub struct PgPool<T = Default>(deadpool_postgres::Pool, PhantomData<T>);

impl PgPool {
  pub async fn get(
    &self,
  ) -> Result<
    deadpool::managed::Object<deadpool_postgres::Manager>,
    deadpool::managed::PoolError<tokio_postgres::Error>,
  > {
    self.0.get().await
  }
}

impl From<deadpool_postgres::Pool> for PgPool {
  fn from(from: deadpool_postgres::Pool) -> PgPool {
    PgPool(from, PhantomData)
  }
}

#[derive(Clone)]
pub struct Default {}

pub struct PgClient<'a, T = Default> {
  inner: PgClientInner<'a>,
  tag: PhantomData<T>,
}

impl<'a, T> PgClient<'a, T> {
  pub fn from_client(client: deadpool_postgres::Client) -> PgClient<'a, T> {
    PgClient {
      inner: PgClientInner::Client(client),
      tag: PhantomData,
    }
  }

  fn from_transaction(transaction: deadpool_postgres::Transaction<'a>) -> PgClient<'a, T> {
    PgClient {
      inner: PgClientInner::Transaction(transaction),
      tag: PhantomData,
    }
  }

  pub async fn from_pool(pool: &PgPool) -> Result<PgClient<'a, T>, PgClientError> {
    Ok(pool.0.get().await.map(|client| Self::from_client(client))?)
  }
}

impl PgClient<'_> {
  pub async fn prepare(&self, query: &str) -> Result<tokio_postgres::Statement, PgClientError> {
    let stmt = match &self.inner {
      PgClientInner::Client(client) => client.prepare_cached(query).await,
      PgClientInner::Transaction(transaction) => transaction.prepare_cached(query).await,
      _ => Err(PgClientError::Internal {
        backtrace: Backtrace::force_capture(),
      })?,
    }
    .map_err(|err| PgClientError::PostgresQuery {
      source: err,
      query: query.to_owned(),
      backtrace: Backtrace::force_capture(),
    });

    stmt
  }

  pub async fn prepare_cached(
    &self,
    query: &str,
  ) -> Result<tokio_postgres::Statement, PgClientError> {
    let stmt = match &self.inner {
      PgClientInner::Client(client) => client.prepare_cached(query).await,
      PgClientInner::Transaction(transaction) => transaction.prepare_cached(query).await,
      _ => Err(PgClientError::Internal {
        backtrace: Backtrace::force_capture(),
      })?,
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
      _ => Err(PgClientError::Internal {
        backtrace: Backtrace::force_capture(),
      })?,
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
      _ => Err(PgClientError::Internal {
        backtrace: Backtrace::force_capture(),
      })?,
    })
  }

  pub async fn simple_query(
    &self,
    query: &str,
  ) -> Result<Vec<tokio_postgres::SimpleQueryMessage>, PgClientError> {
    Ok(match &self.inner {
      PgClientInner::Client(client) => client.simple_query(query).await?,
      PgClientInner::Transaction(transaction) => transaction.simple_query(query).await?,
      _ => Err(PgClientError::Internal {
        backtrace: Backtrace::force_capture(),
      })?,
    })
  }

  pub async fn transaction(&mut self) -> Result<PgClient<'_>, PgClientError> {
    Ok(match &mut self.inner {
      PgClientInner::Client(client) => client.transaction().await?.into(),
      PgClientInner::Transaction(transaction) => transaction.transaction().await?.into(),
      _ => Err(PgClientError::Internal {
        backtrace: Backtrace::force_capture(),
      })?,
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

impl From<deadpool_postgres::Client> for PgClient<'_> {
  fn from(from: deadpool_postgres::Client) -> Self {
    PgClient::from_client(from)
  }
}

impl<'a, T> From<deadpool_postgres::Transaction<'a>> for PgClient<'a, T> {
  fn from(from: deadpool_postgres::Transaction<'a>) -> Self {
    PgClient::<T>::from_transaction(from)
  }
}

pub enum PgClientInner<'a> {
  Client(deadpool_postgres::Client),
  Transaction(deadpool_postgres::Transaction<'a>),
  Empty,
}

impl<'a> FromRequest for PgClient<'a> {
  type Error = PgClientError;
  type Future = Pin<Box<dyn Future<Output = Result<PgClient<'a>, Self::Error>> + 'static>>;

  fn from_request(http: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
    let pool = http
      .app_data::<web::Data<PgPool>>()
      .map(|pool| pool.to_owned());

    Box::pin(async move {
      Ok(
        pool
          .ok_or(PgClientError::Internal {
            backtrace: Backtrace::force_capture(),
          })?
          .0
          .get()
          .map_ok(PgClient::from_client)
          .await?,
      )
    })
  }
}

#[derive(Debug, Error, ResponseError)]
pub enum PgClientError {
  #[error("internal error")]
  Internal { backtrace: Backtrace },
  #[error("postgres error: {source}")]
  Postgres {
    #[from]
    source: tokio_postgres::Error,
    backtrace: Backtrace,
  },
  #[error("postgres query error: {source}\n{query}")]
  PostgresQuery {
    source: tokio_postgres::Error,
    query: String,
    backtrace: Backtrace,
  },
  #[error(transparent)]
  DeadpoolPool(#[from] DeadpoolPoolError),
}

pub type DeadpoolPoolError = deadpool::managed::PoolError<tokio_postgres::Error>;
