use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use std::net::TcpStream;

use crate::routes::TlsMode;

include!(concat!(env!("OUT_DIR"), "/tls_pin.rs"));

pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

static DISABLED: AtomicBool = AtomicBool::new(false);
static CONFIG: OnceLock<Option<Arc<ClientConfig>>> = OnceLock::new();

/// Global escape hatch (`--plaintext`): ignore both the pin and any per-site
/// `tls` flag.
pub fn disable_plaintext() {
    DISABLED.store(true, Ordering::Relaxed);
}

pub fn has_pin() -> bool {
    pin().is_some()
}

fn config() -> Option<Arc<ClientConfig>> {
    CONFIG
        .get_or_init(|| pin().map(tls_client::client_config))
        .clone()
}

/// Wraps a connection according to the site's declared mode.
///
/// `On` and `Auto` differ in what happens when this build has no pin: `On`
/// fails loudly, `Auto` quietly falls back to plaintext so a local plaintext
/// server still works.
pub fn wrap(stream: TcpStream, mode: TlsMode) -> Result<Box<dyn ReadWrite>, ()> {
    if DISABLED.load(Ordering::Relaxed) {
        return Ok(Box::new(stream));
    }
    let config = match mode {
        TlsMode::Off => return Ok(Box::new(stream)),
        TlsMode::On => config().ok_or(())?,
        TlsMode::Auto => match config() {
            Some(config) => config,
            None => return Ok(Box::new(stream)),
        },
    };
    let name = ServerName::try_from("nac.local").map_err(|_| ())?;
    let conn = ClientConnection::new(config, name).map_err(|_| ())?;
    Ok(Box::new(StreamOwned::new(conn, stream)))
}
