use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::routes::TlsMode;
use crate::tls::ReadWrite;

use protocol::{
    decode_server, encode_client, ClientMsg, EmptyReason, Request, ServerMsg, SESSION_LEN,
};

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: u16,
    pub dynamic: bool,
    pub hash: u64,
    pub title: String,
    pub path: String,
    /// Prefix this page answers for, e.g. `/item/`.
    pub matches: Option<String>,
}

/// What a `Resolve` came back with.
#[derive(Clone, Debug)]
pub struct Found {
    pub endpoint: String,
    pub tls: bool,
    pub title: String,
}

pub struct Client {
    pub addr: String,
    pub session: [u8; SESSION_LEN],
    tls: TlsMode,
}

impl Client {
    pub fn with_tls(addr: impl Into<String>, tls: TlsMode) -> Self {
        Self {
            addr: addr.into(),
            session: [0u8; SESSION_LEN],
            tls,
        }
    }

    fn open(&self, request: Request, path: &str) -> Result<Box<dyn ReadWrite>, ()> {
        let stream = TcpStream::connect(&self.addr).map_err(|_| ())?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
        let mut stream = crate::tls::wrap(stream, self.tls)?;

        let msg = ClientMsg {
            session: self.session,
            path: path.to_string(),
            request,
        };
        stream.write_all(&encode_client(&msg)).map_err(|_| ())?;
        Ok(stream)
    }

    fn call(&mut self, request: Request, path: &str) -> Result<ServerMsg, ()> {
        let mut stream = self.open(request, path)?;
        let frame = read_frame(&mut *stream)?;
        decode_server(&frame).ok_or(())
    }

    pub fn hello(&mut self) -> Result<(u16, String), ()> {
        match self.call(Request::Hello, "")? {
            ServerMsg::Hello { version, site } => Ok((version, site)),
            _ => Err(()),
        }
    }

    /// Asks this server where another site lives. `None` means it does not know.
    pub fn resolve(&mut self, name: &str) -> Result<Found, ()> {
        match self.call(Request::Resolve(name.to_string()), "")? {
            ServerMsg::Resolved {
                endpoint,
                tls,
                title,
                ..
            } => Ok(Found {
                endpoint,
                tls,
                title,
            }),
            _ => Err(()),
        }
    }

    pub fn manifest(&mut self) -> Result<Vec<Entry>, ()> {
        match self.call(Request::Manifest, "")? {
            ServerMsg::Manifest { session, entries } => {
                self.session = session;
                Ok(entries
                    .into_iter()
                    .map(|e| Entry {
                        id: e.id,
                        dynamic: e.dynamic,
                        hash: e.hash,
                        title: e.title,
                        path: e.path,
                        matches: e.matches,
                    })
                    .collect())
            }
            _ => Err(()),
        }
    }

    pub fn page(&mut self, id: u16, path: &str) -> Result<String, EmptyReason> {
        match self.call(Request::Page(id), path).map_err(|_| EmptyReason::Failed)? {
            ServerMsg::Page { data, .. } => {
                String::from_utf8(data).map_err(|_| EmptyReason::Failed)
            }
            ServerMsg::Empty(reason) => Err(reason),
            _ => Err(EmptyReason::Failed),
        }
    }

    pub fn submit(
        &mut self,
        id: u16,
        fields: Vec<(String, String)>,
        path: &str,
    ) -> Result<(u16, String), EmptyReason> {
        match self
            .call(Request::Submit(id, fields), path)
            .map_err(|_| EmptyReason::Failed)?
        {
            ServerMsg::Empty(reason) => Err(reason),
            ServerMsg::Page { id, data, .. } => Ok((
                id,
                String::from_utf8(data).map_err(|_| EmptyReason::Failed)?,
            )),
            _ => Err(EmptyReason::Failed),
        }
    }

    pub fn image(&mut self, id: u16) -> Result<Vec<u8>, ()> {
        if let Ok(data) = self.image_stream(id) {
            return Ok(data);
        }
        let mut out = Vec::new();
        loop {
            let offset = out.len() as u32;
            match self.call(Request::Image(id, offset), "")? {
                ServerMsg::Image { total, data, .. } => {
                    if data.is_empty() {
                        break;
                    }
                    out.extend_from_slice(&data);
                    if out.len() >= total as usize {
                        break;
                    }
                }
                _ => return Err(()),
            }
        }
        Ok(out)
    }

    pub fn image_stream(&mut self, id: u16) -> Result<Vec<u8>, ()> {
        let mut stream = self.open(Request::ImageStream(id, 0), "")?;
        let mut out = Vec::new();
        loop {
            let frame = read_frame(&mut stream)?;
            match decode_server(&frame).ok_or(())? {
                ServerMsg::Image { total, data, .. } => {
                    if data.is_empty() {
                        break;
                    }
                    out.extend_from_slice(&data);
                    if out.len() >= total as usize {
                        break;
                    }
                }
                _ => return Err(()),
            }
        }
        Ok(out)
    }
}

fn read_frame<R: Read + ?Sized>(stream: &mut R) -> Result<Vec<u8>, ()> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|_| ())?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > protocol::MAX_FRAME {
        return Err(());
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).map_err(|_| ())?;
    let mut frame = Vec::with_capacity(4 + len);
    frame.extend_from_slice(&len_buf);
    frame.extend_from_slice(&body);
    Ok(frame)
}
