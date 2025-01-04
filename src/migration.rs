use async_trait::async_trait;
use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;

use crate::config::DatabaseConfigError;
use crate::db::DeadpoolPoolError;
use crate::version::Version;

pub struct Migrator<'a> {
  module_name: String,
  migrations: Vec<Box<dyn Migration + 'a>>,
  db_client: deadpool_postgres::Client,
}

lazy_static! {
  static ref MIGRATION_FILE_VERSION: Regex = Regex::new("/v(\\d+)_(\\d+)_(\\d+)+.sql$").unwrap();
}

impl Version {
  pub fn from_filename(filename: &str) -> Result<Self, ()> {
    let captures = MIGRATION_FILE_VERSION.captures(filename).unwrap();

    Ok(Version(
      captures
        .get(1)
        .and_then(|v| v.as_str().parse().ok())
        .ok_or(())?,
      captures
        .get(2)
        .and_then(|v| v.as_str().parse().ok())
        .ok_or(())?,
      captures
        .get(3)
        .and_then(|v| v.as_str().parse().ok())
        .ok_or(())?,
    ))
  }
}

pub const BASE_MODULE_NAME: &str = "fuzion";

impl<'a> Migrator<'a> {
  pub fn new(
    module_name: &str,
    db_client: deadpool_postgres::Client,
    migrations: Vec<Box<dyn Migration + 'a>>,
  ) -> Migrator<'a> {
    Migrator {
      module_name: module_name.into(),
      db_client,
      migrations,
    }
  }

  pub async fn migrate(&mut self) -> Result<(), MigrationError> {
    let version = self.get_version().await?;

    // Find migrations that are newer than current version.
    let migrations: Vec<&Box<dyn Migration>> = self
      .migrations
      .iter()
      .filter(|e| e.version() > version)
      .collect();

    // Perform each one.
    for migration in &migrations {
      info!("Migrating to {:?} ...", &migration.version());

      let mut txn = self.db_client.transaction().await?;

      // If we fail, set a flag
      let result = match migration.do_migration(&mut txn).await {
        Ok(_) => Self::update_version(&txn, &self.module_name, &***migration).await,
        Err(err) => Err(err),
      };

      if result.is_err() {
        error!("Failed migration on version: {:?}", &migration.version());

        return result;
      }

      txn.commit().await?;
    }

    Ok(())
  }

  pub async fn get_version(&self) -> Result<Version, MigrationError> {
    self.initialize_versions().await?;

    // Try to get version, and if we fail, assume the database is uninitialized (0, 0, 0).
    let rows = self
      .db_client
      .query(GET_VERSION_MODULE, &[&self.module_name])
      .await?;

    let version = match rows.first() {
      Some(row) => Version(row.get(0), row.get(1), row.get(2)),
      None => Version(0, 0, 0),
    };

    info!("Current version is: {:?}", &version);

    Ok(version)
  }

  async fn update_version(
    db_client: &deadpool_postgres::Transaction<'_>,
    module_name: &str,
    migration: &dyn Migration,
  ) -> Result<(), MigrationError> {
    let version = migration.version();

    db_client
      .execute(
        UPDATE_MODULE_VERSION,
        &[&module_name, &version.0, &version.1, &version.2],
      )
      .await?;

    Ok(())
  }

  pub async fn initialize_versions(&self) -> Result<(), MigrationError> {
    self.db_client.batch_execute(CREATE_MIGRATOR_SCHEMA).await?;

    // Check if old version table exists.
    let has_old_version: bool = {
      let rows = self.db_client.query(CHECK_OLD_VERSION_EXISTS, &[]).await?;
      rows.first().unwrap().get(0)
    };

    if has_old_version {
      self.db_client.batch_execute(MOVE_VERSION).await?;
    }

    // Check that version table supports modules.
    let has_modules: bool = {
      let rows = self
        .db_client
        .query(CHECK_VERSION_MODULE_EXISTS, &[])
        .await?;
      rows.first().unwrap().get(0)
    };

    if !has_modules {
      // Check that version table supports modules.
      let has_version: bool = {
        let rows = self.db_client.query(CHECK_VERSION_EXISTS, &[]).await?;
        rows.first().unwrap().get(0)
      };

      if has_version {
        self
          .db_client
          .batch_execute(ADD_VERSION_MODULE_COLUMN)
          .await?;
      } else {
        self.db_client.execute(CREATE_VERSION_TABLE, &[]).await?;
      }
    }

    Ok(())
  }
}

#[derive(Debug, Error)]
pub enum MigrationInitError {
  #[error(transparent)]
  DatabaseConfigError(#[from] DatabaseConfigError),
  #[error(transparent)]
  DeadpoolPoolError(#[from] DeadpoolPoolError),
  #[error(transparent)]
  MigrationError(#[from] MigrationError),
}

#[derive(Debug, Error)]
pub enum MigrationError {
  #[error("Could not initialize version table")]
  CouldNotInitializeVersionTable,
  #[error("Base version does not support modules, please upgrade")]
  NoModules,
  #[error("Interactive required.")]
  InteractiveRequired,
  #[error(transparent)]
  IoError(#[from] std::io::Error),
  #[error("Error: {0}")]
  OtherError(String),
  #[error(transparent)]
  Postgres(#[from] tokio_postgres::Error),
}

#[async_trait]
pub trait Migration {
  fn version(&self) -> Version;
  async fn do_migration(
    &self,
    conn: &mut tokio_postgres::Transaction<'_>,
  ) -> Result<(), MigrationError>;
}

#[macro_export]
macro_rules! plain_migration {
  ($arg:literal) => {
    Box::new(fuzion_commons::migration::PlainMigration::new(
      fuzion_commons::version::Version::from_filename($arg).unwrap(),
      include_str!(concat!($arg)),
    ));
  };
}

pub struct PlainMigration {
  version: Version,
  query: &'static str,
}

impl PlainMigration {
  pub fn new(version: Version, query: &'static str) -> Self {
    Self { version, query }
  }
}

#[async_trait]
impl Migration for PlainMigration {
  fn version(&self) -> Version {
    self.version
  }

  async fn do_migration(
    &self,
    conn: &mut tokio_postgres::Transaction<'_>,
  ) -> Result<(), MigrationError> {
    conn.batch_execute(self.query).await?;

    Ok(())
  }
}

const CHECK_VERSION_EXISTS: &str = r#"

SELECT EXISTS (
  SELECT
    1
  FROM information_schema.columns
  WHERE
    table_schema = 'fuzion'
    AND table_name = 'version'
);

"#;

const CHECK_OLD_VERSION_EXISTS: &str = r#"

SELECT EXISTS (
  SELECT
    1
  FROM information_schema.columns
  WHERE
    table_schema = 'public'
    AND table_name = 'version'
);

"#;

const MOVE_VERSION: &str = r#"

ALTER TABLE public.version SET SCHEMA migrator;

"#;

const CHECK_VERSION_MODULE_EXISTS: &str = r#"

SELECT EXISTS (
  SELECT
    1
  FROM information_schema.columns
  WHERE
    table_schema = 'migrator'
    AND table_name = 'version'
    AND column_name = 'module'
);

"#;

const GET_VERSION_MODULE: &str = r#"

SELECT
  major, minor, patch
FROM
  migrator.version
WHERE
  module = $1

"#;

const UPDATE_MODULE_VERSION: &str = r#"

INSERT INTO migrator.version
(module, major, minor, patch)
VALUES
($1, $2, $3, $4)
ON CONFLICT (module)
DO UPDATE SET major = $2, minor = $3, patch = $4;

"#;

const ADD_VERSION_MODULE_COLUMN: &str = r#"

ALTER TABLE migrator.version
  ADD COLUMN module VARCHAR(128);

UPDATE migrator.version
SET module = 'fuzion';

ALTER TABLE migrator.version
  ALTER COLUMN module SET NOT NULL;

ALTER TABLE migrator.version
  ADD PRIMARY KEY (module);

"#;

const CREATE_MIGRATOR_SCHEMA: &str = r#"

CREATE SCHEMA IF NOT EXISTS migrator;

"#;

const CREATE_VERSION_TABLE: &str = r#"

CREATE TABLE migrator.version (
    module varchar(128) NOT NULL PRIMARY KEY,
    major smallint NOT NULL,
    minor smallint NOT NULL,
    patch smallint NOT NULL
);

"#;
