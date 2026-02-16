use std::num::ParseIntError;
use std::process::Stdio;
use std::str::Utf8Error;
use std::time::Duration;

use nix::unistd::Pid;
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::signal;
use tokio::sync::oneshot;
use tokio::time::sleep;
use users::get_user_by_name;

pub const NGINX_PID_FILE: &str = "/tmp/nginx.pid";

pub async fn get_nginx_pid() -> Result<u32, NginxPidError> {
  let content = tokio::fs::read(NGINX_PID_FILE).await?;

  Ok(std::str::from_utf8(&content[..])?.parse()?)
}

#[derive(Debug, Error)]
pub enum NginxPidError {
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  ParseInt(#[from] ParseIntError),
  #[error(transparent)]
  Utf8(#[from] Utf8Error),
}

pub async fn launch() -> Result<(), NginxError> {
  let user = get_user_by_name("root").ok_or(NginxError::NoSuchUser)?;

  let mut child = Command::new("nginx")
    .uid(user.uid())
    .gid(user.primary_group_id())
    .arg("-g")
    .arg("daemon off;")
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn()?;

  tokio::fs::write(
    NGINX_PID_FILE,
    &format!("{}", child.id().ok_or(NginxError::MissingPid)?),
  )
  .await?;

  let (shutdown_tx_tx, mut shutdown_tx_rx) = oneshot::channel::<()>();
  let (shutdown_rx_tx, mut shutdown_rx_rx) = oneshot::channel::<()>();

  fn shutdown(child: &Child) {
    if let Some(pid) = child.id() {
      if let Err(err) =
        nix::sys::signal::kill(Pid::from_raw(pid as i32), nix::sys::signal::Signal::SIGINT)
      {
        println!("Could not kill nginx: {err}");
      }
    }
  }

  tokio::spawn(async move {
    tokio::select! {
      _ = child.wait() => {
        // Nginx shutdown.
      },
      _ = &mut shutdown_tx_rx => {
        // Shutdown signal received.
        shutdown(&child);
      },
    }

    let _ = shutdown_rx_tx.send(());
  });

  tokio::select! {
    _ = signal::ctrl_c() => {
      // Send shutdown signal and wait.
      let _ = shutdown_tx_tx.send(());
      let _ = shutdown_rx_rx.await;
    }
    _ = &mut shutdown_rx_rx => {
      // Nginx shut down.
    }
  }

  Ok(())
}

pub async fn kill() -> Result<(), NginxError> {
  let nginx_pid = get_nginx_pid().await?;

  if let Err(err) = nix::sys::signal::kill(
    Pid::from_raw(nginx_pid as i32),
    nix::sys::signal::Signal::SIGINT,
  ) {
    println!("Could not SIGINT Nginx: {err}");

    return Ok(());
  }

  sleep(Duration::from_secs(10)).await;

  if let Err(err) = nix::sys::signal::kill(
    Pid::from_raw(nginx_pid as i32),
    nix::sys::signal::Signal::SIGKILL,
  ) {
    println!("Could not SIGKILL Nginx: {err}");
  }

  Ok(())
}

#[derive(Debug, Error)]
pub enum NginxError {
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error("Missing pid")]
  MissingPid,
  #[error(transparent)]
  NginxPid(#[from] NginxPidError),
  #[error("No such user")]
  NoSuchUser,
}
