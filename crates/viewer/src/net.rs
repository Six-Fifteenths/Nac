use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use protocol::{decode_server, encode_client, ClientMsg, Request, ServerMsg, SESSION_LEN};

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: u16,
    pub dynamic: bool,
    pub hash: u64,
    pub title: String,
}

pub struct Client {
    pub addr: String,
    pub session: [u8; SESSION_LEN],
}

impl Client {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            session: [0u8; SESSION_LEN],
        }
    }

    fn call(&mut self, request: Request) -> Result<ServerMsg, ()> {
        let mut stream = TcpStream::connect(&self.addr).map_err(|_| ())?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

        let msg = ClientMsg {
            session: self.session,
            request,
        };
        stream.write_all(&encode_client(&msg)).map_err(|_| ())?;

        let frame = read_frame(&mut stream)?;
        decode_server(&frame).ok_or(())
    }

    pub fn manifest(&mut self) -> Result<Vec<Entry>, ()> {
        match self.call(Request::Manifest)? {
            ServerMsg::Manifest { session, entries } => {
                self.session = session;
                Ok(entries
                    .into_iter()
                    .map(|e| Entry {
                        id: e.id,
                        dynamic: e.dynamic,
                        hash: e.hash,
                        title: e.title,
                    })
                    .collect())
            }
            _ => Err(()),
        }
    }

    pub fn page(&mut self, id: u16) -> Result<String, ()> {
        match self.call(Request::Page(id))? {
            ServerMsg::Page { data, .. } => String::from_utf8(data).map_err(|_| ()),
            _ => Err(()),
        }
    }

    pub fn submit(&mut self, id: u16, fields: Vec<(String, String)>) -> Result<(u16, String), ()> {
        match self.call(Request::Submit(id, fields))? {
            ServerMsg::Page { id, data, .. } => Ok((
                id,
                String::from_utf8(data).map_err(|_| ())?,
            )),
            _ => Err(()),
        }
    }

    pub fn image(&mut self, id: u16) -> Result<Vec<u8>, ()> {
        let mut out = Vec::new();
        loop {
            let offset = out.len() as u32;
            match self.call(Request::Image(id, offset))? {
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

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, ()> {
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
