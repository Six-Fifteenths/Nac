//! Operator-only convenience: open the SSH tunnel for a site automatically.
//!
//! The distributed viewer is built without the `admin` feature, so this file is
//! not compiled into it at all — end users never get code that can spawn a
//! shell command, and there is nothing to abuse if the binary is unpacked.
//!
//! The tunnel still is SSH: the same forwarding an operator would type by hand.
//! It runs `ssh -N -L <port>:<host>:<port> <dest>` as a child process, waits
//! until the local port accepts a connection, and kills the child on drop.

use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::routes::Site;

/// How long to wait for `ssh` to bring the forward up.
const READY_TIMEOUT: Duration = Duration::from_secs(12);

pub struct Tunnel {
    child: Option<Child>,
}

impl Tunnel {
    /// Returns a tunnel for `site`, or `None` when one is not needed.
    ///
    /// An already-working forward is reused: if the operator has their own
    /// `ssh -L` running, we stay out of the way.
    pub fn ensure(site: &Site) -> Result<Option<Tunnel>, String> {
        let Some(dest) = site.ssh.as_ref() else {
            return Ok(None);
        };
        let (host, port) = split(&site.endpoint).ok_or_else(|| {
            format!("站点 {} 的端点不是 host:port：{}", site.name, site.endpoint)
        })?;

        if reachable(&site.endpoint) {
            return Ok(Some(Tunnel { child: None }));
        }

        let forward = format!("{port}:{host}:{port}");
        let mut command = Command::new("ssh");
        command
            .args(["-N"])
            // Do not hang on a passphrase prompt: fail and say so instead.
            .args(["-o", "BatchMode=yes"])
            .args(["-o", "ServerAliveInterval=30"])
            .args(["-o", "ExitOnForwardFailure=yes"])
            .args(["-o", "StrictHostKeyChecking=accept-new"])
            .args(["-L", &forward])
            .arg(dest)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        // ssh is a console program: when the viewer itself has no console (the
        // GUI build), spawning it would pop up a black window. Suppress that.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let child = command
            .spawn()
            .map_err(|e| format!("无法启动 ssh（需要系统里有 OpenSSH 客户端）：{e}"))?;

        let mut tunnel = Tunnel {
            child: Some(child),
        };

        if wait_ready(&site.endpoint) {
            return Ok(Some(tunnel));
        }

        let detail = tunnel.take_stderr();
        Err(format!(
            "ssh 隧道没能建立（{dest} → {forward}）{detail}。若密钥有口令，请先启动 ssh-agent。"
        ))
    }

    fn take_stderr(&mut self) -> String {
        let Some(mut child) = self.child.take() else {
            return String::new();
        };
        let _ = child.kill();
        match child.wait_with_output() {
            Ok(out) if !out.stderr.is_empty() => {
                let text = String::from_utf8_lossy(&out.stderr);
                let line = text
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if line.is_empty() {
                    String::new()
                } else {
                    format!("：{line}")
                }
            }
            _ => String::new(),
        }
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn split(endpoint: &str) -> Option<(String, u16)> {
    let (host, port) = endpoint.rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?))
}

fn reachable(endpoint: &str) -> bool {
    let Some(addrs) = endpoint.to_socket_addrs().ok() else {
        return false;
    };
    for addr in addrs {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok() {
            return true;
        }
    }
    false
}

fn wait_ready(endpoint: &str) -> bool {
    let deadline = Instant::now() + READY_TIMEOUT;
    while Instant::now() < deadline {
        if reachable(endpoint) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}