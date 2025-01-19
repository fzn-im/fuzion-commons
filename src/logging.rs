use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use file_rotate::compression::Compression;
use file_rotate::suffix::{AppendTimestamp, DateFrom, FileLimit};
use file_rotate::{ContentLimit, FileRotate};
use lazy_static::lazy_static;
use slog::Drain;

use crate::config::LoggingConfig;

lazy_static! {
  static ref LOG_GUARD: Arc<Mutex<Option<LoggingGuard>>> = Arc::new(Mutex::new(None));
}

pub struct LoggingGuard {
  _scope_guard: slog_scope::GlobalLoggerGuard,
}

lazy_static! {
  static ref FILTERED_MODULES: HashSet<&'static str> = {
    let mut modules = HashSet::new();
    modules.insert("mio");
    modules.insert("postgres");
    modules.insert("tokio_io");
    modules.insert("tokio_postgres");
    modules.insert("tokio_reactor");
    modules
  };
}

fn filter_records(record: &slog::Record) -> bool {
  let mut pieces = record.module().split("::");
  let lcrate = pieces.next();
  if let Some(lcrate) = lcrate {
    if FILTERED_MODULES.contains(lcrate) {
      return false;
    }
    // println!("{}", lcrate);
  }
  true
}

pub fn init(config: &LoggingConfig) {
  let values = slog_o!("place" =>
    slog::FnValue(move |info| {
      format!(
        "{}:{} {}",
        info.file(),
        info.line(),
        info.module(),
      )
    })
  );

  let _scope_guard = match (&config.log_file, config.log_to_stdout) {
    (None, true) => {
      let decorator = slog_term::TermDecorator::new().build();
      let drain = slog_term::FullFormat::new(decorator).build().fuse();
      let drain = slog_async::Async::new(drain.fuse()).build().fuse();
      let drain = slog::LevelFilter::new(drain, config.log_level)
        .filter(filter_records)
        .fuse();
      let logger = slog::Logger::root(drain, values);
      Some(slog_scope::set_global_logger(logger))
    }
    (Some(log_file), false) => {
      let file = FileRotate::new(
        log_file,
        AppendTimestamp::with_format("%Y%m%d", FileLimit::MaxFiles(5), DateFrom::DateYesterday),
        ContentLimit::Bytes(100_000_000),
        Compression::OnRotate(2),
        #[cfg(unix)]
        None,
      );

      let decorator = slog_term::PlainDecorator::new(file);
      let drain = slog_term::FullFormat::new(decorator).build().fuse();
      let drain = slog_async::Async::new(drain.fuse()).build().fuse();
      let drain = slog::LevelFilter::new(drain, config.log_level)
        .filter(filter_records)
        .fuse();

      let logger = slog::Logger::root(drain, values);
      Some(slog_scope::set_global_logger(logger))
    }
    (Some(log_file), true) => {
      let decorator = slog_term::TermDecorator::new().build();
      let drain = slog_term::FullFormat::new(decorator).build().fuse();
      let drain = slog_async::Async::new(drain.fuse()).build().fuse();
      let drain = slog::LevelFilter::new(drain, config.log_level)
        .filter(filter_records)
        .fuse();

      let file = FileRotate::new(
        log_file,
        AppendTimestamp::with_format("%Y%m%d", FileLimit::MaxFiles(5), DateFrom::DateYesterday),
        ContentLimit::Bytes(100_000_000),
        Compression::OnRotate(2),
        #[cfg(unix)]
        None,
      );

      let decorator = slog_term::PlainDecorator::new(file);
      let drain2 = slog_term::FullFormat::new(decorator).build().fuse();
      let drain2 = slog_async::Async::new(drain2.fuse()).build().fuse();
      let drain2 = slog::LevelFilter::new(drain2, config.log_level)
        .filter(filter_records)
        .fuse();

      let drain = slog::Duplicate(drain, drain2).fuse();

      let logger = slog::Logger::root(drain, values);
      Some(slog_scope::set_global_logger(logger))
    }
    _ => None,
  };

  if let Some(_scope_guard) = _scope_guard {
    slog_stdlog::init().unwrap();

    info!("log_level: {}", config.log_level);
    info!("log_file: {:?}", &config.log_file);
    info!("log_to_stdout: {}", config.log_to_stdout);

    let mut log_guard = LOG_GUARD.lock().unwrap();
    *log_guard = Some(LoggingGuard { _scope_guard });
  }
}
