use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command};
use std::sync::mpsc::channel;
use std::thread::{spawn, JoinHandle};

use crate::{ConnectionSettings, WithClient};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use postgres::Client;
use tempfile::TempDir;

pub struct TempServer {
    dbpath: Option<TempDir>,
    child: Child,
    reader: Option<JoinHandle<()>>,
    logger: Option<JoinHandle<()>>,
    connection_settings: ConnectionSettings,
}

impl TempServer {
    pub fn new(postgres_options: &str, initdb_options: &[String]) -> Result<Self> {
        let port = find_free_port_on_localhost()?;
        check_required_postgres_commands()?;
        let dbpath = TempDir::new()?;
        let mut superuser_password = String::new();
        while superuser_password.len() < 20 {
            let rand_byte: u8 = rand::random();
            if rand_byte.is_ascii_alphanumeric() {
                superuser_password.push(rand_byte as char);
            }
        }
        let mut pwfile = tempfile::NamedTempFile::new()?;
        pwfile.write_all(superuser_password.as_bytes())?;

        let mut initdb = Command::new("initdb");
        initdb
            .arg("-D")
            .arg(dbpath.path())
            .arg("--pwfile")
            .arg(pwfile.path())
            .arg("--username")
            .arg("postgres");
        for option in initdb_options {
            initdb.arg(option);
        }
        let initdb = initdb.output()?;

        if !initdb.status.success() {
            return Err(anyhow!("initdb failed: {initdb:?}",));
        }

        let mut pg = Command::new("pg_ctl");
        pg.arg("start")
            .arg("-D")
            .arg(dbpath.path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .arg("-o")
            .arg(format!("-p {port}"))
            .arg("-o")
            .arg(format!(
                "-c unix_socket_directories={}",
                dbpath.path().to_string_lossy()
            ))
            .arg("-o")
            .arg(postgres_options);

        let mut child = pg.spawn()?;
        let (sender, receiver) = channel();
        let stdout = child.stdout.take().context("Unable to take stdout")?;

        let reader = spawn(move || {
            let r = BufReader::new(stdout);
            for line in r.lines().map_while(Result::ok) {
                if let Err(e) = sender.send(line) {
                    error!("Unable to send log: {e:?}");
                    break;
                }
            }
        });

        loop {
            let log = receiver.recv()?;
            info!("postgres log: {log}");
            if log.contains("ready to accept") {
                break;
            }
        }

        let logger = spawn(move || loop {
            let log = receiver.recv();
            match log {
                Ok(l) => {
                    debug!("postgres log: {}", l);
                }
                Err(e) => {
                    warn!("Unable to receive log from postgres: {e:}");
                    break;
                }
            }
        });

        Ok(TempServer {
            dbpath: Some(dbpath),
            child,
            reader: Some(reader),
            logger: Some(logger),
            connection_settings: ConnectionSettings::new(
                "postgres".to_string(),
                "postgres".to_string(),
                "localhost".to_string(),
                port,
                superuser_password.clone(),
            ),
        })
    }
}

fn check_required_postgres_commands() -> Result<()> {
    let required = ["initdb", "postgres"];
    for command in required.iter() {
        Command::new(command)
            .arg("--help")
            .output()
            .map_err(|err| {
                anyhow!(
                    "This functionality requires {command}, but it isn't available on PATH: {err}"
                )
            })?;
    }
    Ok(())
}

fn find_free_port_on_localhost() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

impl WithClient for TempServer {
    fn with_client<T>(&mut self, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T> {
        self.connection_settings.with_client(f)
    }
}

impl Drop for TempServer {
    fn drop(&mut self) {
        debug!("Dropping TempServer at {:?}", &self.dbpath);
        let path_name = self.dbpath.as_ref().map(|d| d.path());

        // This matches unless drop has already run
        if let Some(path_name) = path_name {
            let r = Command::new("pg_ctl")
                .arg("stop")
                .arg("-D")
                .arg(path_name)
                .arg("-m")
                .arg("immediate")
                .output();

            if let Err(problem) = r {
                warn!("Trouble stopping postgres: {problem:?}");
            }
        }

        let child = self.child.kill();
        match child {
            Err(e) => info!("Failed to stop postgres: {:?}", e),
            Ok(()) => {
                debug!("Stopped postgres, deleting {:?}", self.dbpath);
                if let Some(dbpath) = self.dbpath.take() {
                    if let Err(e) = dbpath.close() {
                        warn!("Failed to delete tempdir: {:?}", e);
                    }
                }
            }
        }

        // These both match since this is the only place where we .take()
        // and drop can't run twice
        if let Some(reader) = self.reader.take() {
            if let Err(e) = reader.join() {
                warn!("Unable to join reader thread: {e:?}");
            }
        }
        if let Some(logger) = self.logger.take() {
            if let Err(e) = logger.join() {
                warn!("Unable to join logger thread: {e:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn temp_server_cleans_up_when_leaving_scope() {
        env_logger::init();
        fn inner() -> String {
            let mut s = TempServer::new("", &[]).unwrap();
            let rows: Vec<_> = s
                .with_client(|client| Ok(client.query("SELECT 1 + 1", &[]).unwrap()))
                .unwrap();
            assert!(!rows.is_empty());
            s.dbpath
                .as_ref()
                .unwrap()
                .path()
                .to_string_lossy()
                .to_string()
        }
        let path = inner();
        assert!(!Path::new(&path).exists());
    }
}
