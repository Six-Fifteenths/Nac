use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use std::net::TcpStream;

include!(concat!(env!("OUT_DIR"), "/tls_pin.rs"));

pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

static DISABLED: AtomicBool = AtomicBool::new(false);
static CONFIG: OnceLock<Option<Arc<ClientConfig>>> = OnceLock::new();

pub fn disable_plaintext() {
    DISABLED.store(true, Ordering::Relaxed);
}

fn config() -> Option<Arc<ClientConfig>> {
    CONFIG
        .get_or_init(|| pin().map(tls_client::client_config))
        .clone()
}

pub fn wrap(stream: TcpStream) -> Result<Box<dyn ReadWrite>, ()> {
    if DISABLED.load(Ordering::Relaxed) {
        return Ok(Box::new(stream));
    }
    let Some(config) = config() else {
        return Ok(Box::new(stream));
    };
    let name = ServerName::try_from("nac.local").map_err(|_| ())?;
    let conn = ClientConnection::new(config, name).map_err(|_| ())?;
    Ok(Box::new(StreamOwned::new(conn, stream)))
}
