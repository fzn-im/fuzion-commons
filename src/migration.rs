use async_trait::async_trait;
use thiserror::Error;

use crate::version::Version;

pub struct Migrator<'a> {
  module_name: String,
  migrations: Vec<Box<dyn Migration + 'a>>,
  db_client: deadpool_postgres::Client,
}

pub const BASE_MODULE_NAME: &'static str = "base";

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
    let mut iter = migrations.iter();
    let mut next = iter.next();
    let mut had_error = false;

    while next.is_some() && !had_error {
      let migration = next.unwrap();
      info!("Migrating to {:?} ...", &migration.version());

      let mut txn = self.db_client.transaction().await.unwrap();

      // If we fail, set a flag
      match migration.do_migration(&mut txn).await {
        Ok(_) => {
          if Self::update_version(&txn, &self.module_name, &***migration)
            .await
            .is_err()
          {
            had_error = true;
          }
        }
        Err(_) => {
          had_error = true;
        }
      }

      if had_error {
        error!("Failed migration on version: {:?}", &migration.version());
      } else if txn.commit().await.is_err() {
        error!("Failed to commit migration");

        had_error = true
      }

      next = iter.next();
    }

    if had_error {
      panic!("Could not perform migrations.");
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

    let version = match rows.get(0) {
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
    // Check that version table supports modules.
    let has_modules: bool = {
      let rows = self
        .db_client
        .query(CHECK_VERSION_MODULE_EXISTS, &[])
        .await?;
      rows.get(0).unwrap().get(0)
    };

    if !has_modules {
      // Check that version table supports modules.
      let has_version: bool = {
        let rows = self.db_client.query(CHECK_VERSION_EXISTS, &[]).await?;
        rows.get(0).unwrap().get(0)
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
pub enum MigrationError {
  #[error("could not initialize version table")]
  CouldNotInitializeVersionTable,
  #[error("base version does not support modules, please upgrade")]
  NoModules,
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

const CHECK_VERSION_EXISTS: &'static str = r#"

SELECT EXISTS (
  SELECT
    1
  FROM information_schema.columns
  WHERE
    table_schema = 'public'
    AND table_name = 'version'
);

"#;

const CHECK_VERSION_MODULE_EXISTS: &'static str = r#"

SELECT EXISTS (
  SELECT
    1
  FROM information_schema.columns
  WHERE
    table_schema = 'public'
    AND table_name = 'version'
    AND column_name = 'module'
);

"#;

const GET_VERSION_MODULE: &'static str = r#"

SELECT
  major, minor, patch
FROM
  public.version
WHERE
  module = $1

"#;

const UPDATE_MODULE_VERSION: &'static str = r#"

INSERT INTO public.version
(module, major, minor, patch)
VALUES
($1, $2, $3, $4)
ON CONFLICT (module)
DO UPDATE SET major = $2, minor = $3, patch = $4;

"#;

const ADD_VERSION_MODULE_COLUMN: &'static str = r#"

ALTER TABLE public.version
  ADD COLUMN module VARCHAR(128);

UPDATE public.version
SET module = 'base';

ALTER TABLE public.version
  ALTER COLUMN module SET NOT NULL;

ALTER TABLE public.version
  ADD PRIMARY KEY (module);

"#;

const CREATE_VERSION_TABLE: &'static str = r#"

CREATE TABLE public.version (
    module varchar(128) NOT NULL PRIMARY KEY,
    major smallint NOT NULL,
    minor smallint NOT NULL,
    patch smallint NOT NULL
);

"#;
