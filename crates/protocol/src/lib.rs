//! Wire format.
//!
//! ```text
//! frame := u32be len | u8 type | payload      (len = 1 + payload.len(), len <= MAX_FRAME)
//! ```
//!
//! No magic bytes, no version field: both are fingerprints.
//! Every decoder returns `None` on anything malformed, and never panics.

pub const VERSION: u16 = 5;
pub const MAX_FRAME: usize = 32768;
pub const CHUNK: usize = 16384;
pub const SESSION_LEN: usize = 16;

/// 空响应为什么空。0 是「以前没有这个字段」，读成 Missing 最保守。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyReason {
    /// 老服务器没发这个字节：分不清是哪一种，客户端只能说一句笼统的。
    Unknown = 0,
    /// 地址没有对应的页，或者页面自己 `nac.stop()` 了。
    Missing = 1,
    /// 页面跑失败了：崩了、超时了、输出超上限。
    Failed = 2,
    /// 服务器不认这个请求（协议不对、重定向成环）。
    Refused = 3,
}

impl EmptyReason {
    fn from_byte(b: u8) -> EmptyReason {
        match b {
            1 => EmptyReason::Missing,
            2 => EmptyReason::Failed,
            3 => EmptyReason::Refused,
            _ => EmptyReason::Unknown,
        }
    }
}

const T_REQ: u8 = 0x01;
const T_SUBMIT: u8 = 0x02;
const T_MANIFEST: u8 = 0x81;
const T_PAGE: u8 = 0x82;
const T_IMAGE: u8 = 0x83;
const T_EMPTY: u8 = 0x84;
const T_HELLO: u8 = 0x85;
const T_RESOLVED: u8 = 0x86;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub id: u16,
    pub dynamic: bool,
    pub hash: u64,
    pub title: String,
    /// Path shown to the reader, e.g. `/` or `/notes/`.
    pub path: String,
    /// Prefix this page answers for, e.g. `/item/`. Present only on dynamic
    /// pages that handle a whole family of addresses; the rest of the address is
    /// handed to the page as `NAC_PATH`.
    pub matches: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Hello,
    Manifest,
    Page(u16),
    Image(u16, u32),
    ImageStream(u16, u32),
    Submit(u16, Vec<(String, String)>),
    /// Asks a server where another site lives. Every server answers from its own
    /// registry, so a network can grow by sites knowing about each other instead
    /// of by one central table.
    Resolve(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientMsg {
    pub session: [u8; SESSION_LEN],
    /// The address the reader is asking for, e.g. `/item/123`. Empty when the
    /// request does not depend on it. A page that answers a whole prefix needs
    /// this to know which member of the family was asked for.
    pub path: String,
    pub request: Request,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMsg {
    Hello {
        version: u16,
        /// The name this server goes by, so the client never has to guess.
        site: String,
    },
    /// Answer to `Request::Resolve`.
    Resolved {
        name: String,
        endpoint: String,
        tls: bool,
        title: String,
    },
    Manifest {
        session: [u8; SESSION_LEN],
        entries: Vec<ManifestEntry>,
    },
    Page {
        id: u16,
        total: u32,
        offset: u32,
        data: Vec<u8>,
    },
    Image {
        id: u16,
        total: u32,
        offset: u32,
        data: Vec<u8>,
    },
    /// 服务器没有内容可给。`reason` 说清是哪一种「没有」—— 老客户端会忽略
    /// 这个字节，仍然只显示一句「加载失败」。
    Empty(EmptyReason),
}

// ---------------------------------------------------------------- encoding

fn frame(msg_type: u8, payload: &[u8]) -> Vec<u8> {
    let len = 1 + payload.len();
    let mut out = Vec::with_capacity(4 + len);
    out.extend_from_slice(&(len as u32).to_be_bytes());
    out.push(msg_type);
    out.extend_from_slice(payload);
    out
}

pub fn encode_client(msg: &ClientMsg) -> Vec<u8> {
    let mut p = Vec::with_capacity(64);
    p.extend_from_slice(&msg.session);
    // The address travels with every request: a page that answers a prefix needs
    // to know which member of the family was asked for, and a form submission
    // needs to come back to the same one.
    let pb = msg.path.as_bytes();
    let pl = pb.len().min(u16::MAX as usize) as u16;
    p.extend_from_slice(&pl.to_be_bytes());
    p.extend_from_slice(&pb[..pl as usize]);

    match &msg.request {
        Request::Hello => {
            p.push(3);
            frame(T_REQ, &p)
        }
        Request::Manifest => {
            p.push(0);
            frame(T_REQ, &p)
        }
        Request::Page(id) => {
            p.push(1);
            p.extend_from_slice(&id.to_be_bytes());
            frame(T_REQ, &p)
        }
        Request::Image(id, offset) => {
            p.push(2);
            p.extend_from_slice(&id.to_be_bytes());
            p.extend_from_slice(&offset.to_be_bytes());
            frame(T_REQ, &p)
        }
        Request::ImageStream(id, offset) => {
            p.push(4);
            p.extend_from_slice(&id.to_be_bytes());
            p.extend_from_slice(&offset.to_be_bytes());
            frame(T_REQ, &p)
        }
        Request::Submit(id, fields) => {
            p.extend_from_slice(&id.to_be_bytes());
            p.extend_from_slice(&(fields.len() as u16).to_be_bytes());
            for (k, v) in fields {
                let kb = k.as_bytes();
                let vb = v.as_bytes();
                let kl = kb.len().min(255) as u8;
                p.push(kl);
                p.extend_from_slice(&kb[..kl as usize]);
                p.extend_from_slice(&(vb.len() as u32).to_be_bytes());
                p.extend_from_slice(vb);
            }
            frame(T_SUBMIT, &p)
        }
        Request::Resolve(name) => {
            p.push(5);
            let nb = name.as_bytes();
            let nl = nb.len().min(u16::MAX as usize) as u16;
            p.extend_from_slice(&nl.to_be_bytes());
            p.extend_from_slice(&nb[..nl as usize]);
            frame(T_REQ, &p)
        }
    }
}

pub fn encode_server(msg: &ServerMsg) -> Vec<u8> {
    match msg {
        ServerMsg::Hello { version, site } => {
            let mut p = Vec::with_capacity(4 + site.len());
            p.extend_from_slice(&version.to_be_bytes());
            let sb = site.as_bytes();
            let sl = sb.len().min(u16::MAX as usize) as u16;
            p.extend_from_slice(&sl.to_be_bytes());
            p.extend_from_slice(&sb[..sl as usize]);
            frame(T_HELLO, &p)
        }
        ServerMsg::Resolved {
            name,
            endpoint,
            tls,
            title,
        } => {
            let mut p = Vec::with_capacity(8 + name.len() + endpoint.len() + title.len());
            for text in [name, endpoint] {
                let b = text.as_bytes();
                let l = b.len().min(u16::MAX as usize) as u16;
                p.extend_from_slice(&l.to_be_bytes());
                p.extend_from_slice(&b[..l as usize]);
            }
            p.push(u8::from(*tls));
            let tb = title.as_bytes();
            let tl = tb.len().min(u16::MAX as usize) as u16;
            p.extend_from_slice(&tl.to_be_bytes());
            p.extend_from_slice(&tb[..tl as usize]);
            frame(T_RESOLVED, &p)
        }
        ServerMsg::Manifest { session, entries } => {
            let mut p = Vec::with_capacity(32 + entries.len() * 32);
            p.extend_from_slice(session);
            p.extend_from_slice(&(entries.len() as u16).to_be_bytes());
            for e in entries {
                p.extend_from_slice(&e.id.to_be_bytes());
                p.push(u8::from(e.dynamic));
                p.extend_from_slice(&e.hash.to_be_bytes());
                let tb = e.title.as_bytes();
                let tl = tb.len().min(u16::MAX as usize) as u16;
                p.extend_from_slice(&tl.to_be_bytes());
                p.extend_from_slice(&tb[..tl as usize]);
                let pb = e.path.as_bytes();
                let pl = pb.len().min(u16::MAX as usize) as u16;
                p.extend_from_slice(&pl.to_be_bytes());
                p.extend_from_slice(&pb[..pl as usize]);
                let mb = e.matches.as_deref().unwrap_or("").as_bytes();
                let ml = mb.len().min(u16::MAX as usize) as u16;
                p.extend_from_slice(&ml.to_be_bytes());
                p.extend_from_slice(&mb[..ml as usize]);
            }
            frame(T_MANIFEST, &p)
        }
        ServerMsg::Page {
            id,
            total,
            offset,
            data,
        } => {
            let mut p = Vec::with_capacity(14 + data.len());
            p.extend_from_slice(&id.to_be_bytes());
            p.extend_from_slice(&total.to_be_bytes());
            p.extend_from_slice(&offset.to_be_bytes());
            p.extend_from_slice(&(data.len() as u32).to_be_bytes());
            p.extend_from_slice(data);
            frame(T_PAGE, &p)
        }
        ServerMsg::Image {
            id,
            total,
            offset,
            data,
        } => {
            let mut p = Vec::with_capacity(14 + data.len());
            p.extend_from_slice(&id.to_be_bytes());
            p.extend_from_slice(&total.to_be_bytes());
            p.extend_from_slice(&offset.to_be_bytes());
            p.extend_from_slice(&(data.len() as u32).to_be_bytes());
            p.extend_from_slice(data);
            frame(T_IMAGE, &p)
        }
        ServerMsg::Empty(reason) => {
            let mut payload = vec![0u8; SESSION_LEN];
            payload.push(*reason as u8);
            frame(T_EMPTY, &payload)
        }
    }
}

// ---------------------------------------------------------------- decoding

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, p: 0 }
    }
    fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.p)?;
        self.p += 1;
        Some(v)
    }
    fn u16(&mut self) -> Option<u16> {
        let end = self.p.checked_add(2)?;
        let s = self.b.get(self.p..end)?;
        self.p = end;
        Some(u16::from_be_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        let end = self.p.checked_add(4)?;
        let s = self.b.get(self.p..end)?;
        self.p = end;
        Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u64(&mut self) -> Option<u64> {
        let end = self.p.checked_add(8)?;
        let s = self.b.get(self.p..end)?;
        self.p = end;
        Some(u64::from_be_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }
    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.p.checked_add(n)?;
        let s = self.b.get(self.p..end)?;
        self.p = end;
        Some(s)
    }
    fn string(&mut self, n: usize) -> Option<String> {
        String::from_utf8(self.bytes(n)?.to_vec()).ok()
    }
    fn done(&self) -> bool {
        self.p == self.b.len()
    }
}

fn split(buf: &[u8]) -> Option<(u8, &[u8])> {
    if buf.len() < 5 {
        return None;
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if len == 0 || len > MAX_FRAME || buf.len() != 4 + len {
        return None;
    }
    Some((buf[4], &buf[5..]))
}

pub fn decode_client(buf: &[u8]) -> Option<ClientMsg> {
    let (t, p) = split(buf)?;
    let mut r = Reader::new(p);
    let session: [u8; SESSION_LEN] = r.bytes(SESSION_LEN)?.try_into().ok()?;
    let pl = r.u16()? as usize;
    let path = r.string(pl)?;

    let request = match t {
        T_REQ => match r.u8()? {
            3 => {
                if !r.done() {
                    return None;
                }
                Request::Hello
            }
            0 => {
                if !r.done() {
                    return None;
                }
                Request::Manifest
            }
            1 => {
                let id = r.u16()?;
                if !r.done() {
                    return None;
                }
                Request::Page(id)
            }
            2 => {
                let id = r.u16()?;
                let off = r.u32()?;
                if !r.done() {
                    return None;
                }
                Request::Image(id, off)
            }
            4 => {
                let id = r.u16()?;
                let off = r.u32()?;
                if !r.done() {
                    return None;
                }
                Request::ImageStream(id, off)
            }
            5 => {
                let nl = r.u16()? as usize;
                let name = r.string(nl)?;
                if !r.done() {
                    return None;
                }
                Request::Resolve(name)
            }
            _ => return None,
        },
        T_SUBMIT => {
            let id = r.u16()?;
            let count = r.u16()?;
            let mut fields = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let kl = r.u8()? as usize;
                let k = r.string(kl)?;
                let vl = r.u32()? as usize;
                let v = r.string(vl)?;
                fields.push((k, v));
            }
            if !r.done() {
                return None;
            }
            Request::Submit(id, fields)
        }
        _ => return None,
    };

    Some(ClientMsg {
        session,
        path,
        request,
    })
}

pub fn decode_server(buf: &[u8]) -> Option<ServerMsg> {
    let (t, p) = split(buf)?;
    let mut r = Reader::new(p);

    match t {
        T_HELLO => {
            let version = r.u16()?;
            let sl = r.u16()? as usize;
            let site = r.string(sl)?;
            if !r.done() {
                return None;
            }
            Some(ServerMsg::Hello { version, site })
        }
        T_RESOLVED => {
            let nl = r.u16()? as usize;
            let name = r.string(nl)?;
            let el = r.u16()? as usize;
            let endpoint = r.string(el)?;
            let tls = r.u8()? & 1 == 1;
            let tl = r.u16()? as usize;
            let title = r.string(tl)?;
            if !r.done() {
                return None;
            }
            Some(ServerMsg::Resolved {
                name,
                endpoint,
                tls,
                title,
            })
        }
        T_MANIFEST => {
            let session: [u8; SESSION_LEN] = r.bytes(SESSION_LEN)?.try_into().ok()?;
            let n = r.u16()? as usize;
            let mut entries = Vec::with_capacity(n);
            for _ in 0..n {
                let id = r.u16()?;
                let flags = r.u8()?;
                let hash = r.u64()?;
                let tl = r.u16()? as usize;
                let title = r.string(tl)?;
                let pl = r.u16()? as usize;
                let path = r.string(pl)?;
                let ml = r.u16()? as usize;
                let matches = if ml == 0 {
                    None
                } else {
                    Some(r.string(ml)?)
                };
                entries.push(ManifestEntry {
                    id,
                    dynamic: flags & 1 == 1,
                    hash,
                    title,
                    path,
                    matches,
                });
            }
            if !r.done() {
                return None;
            }
            Some(ServerMsg::Manifest { session, entries })
        }
        T_PAGE | T_IMAGE => {
            let id = r.u16()?;
            let total = r.u32()?;
            let offset = r.u32()?;
            let len = r.u32()? as usize;
            let data = r.bytes(len)?.to_vec();
            if !r.done() {
                return None;
            }
            if t == T_PAGE {
                Some(ServerMsg::Page {
                    id,
                    total,
                    offset,
                    data,
                })
            } else {
                Some(ServerMsg::Image {
                    id,
                    total,
                    offset,
                    data,
                })
            }
        }
        T_EMPTY => {
            // 老服务器只发会话号；新服务器多发一个原因字节。两种都认。
            let _session = r.bytes(SESSION_LEN)?;
            let reason = if r.done() {
                EmptyReason::Unknown
            } else {
                EmptyReason::from_byte(r.u8()?)
            };
            Some(ServerMsg::Empty(reason))
        }
        _ => None,
    }
}

/// Deterministic, non-cryptographic hash used only for cache validation.
pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> [u8; SESSION_LEN] {
        let mut x = [0u8; SESSION_LEN];
        for (i, b) in x.iter_mut().enumerate() {
            *b = i as u8;
        }
        x
    }

    #[test]
    fn client_roundtrip() {
        let cases = vec![
            Request::Hello,
            Request::Manifest,
            Request::Page(7),
            Request::Image(3, 4096),
            Request::ImageStream(3, 0),
            Request::Submit(
                7,
                vec![
                    ("name".into(), "alice".into()),
                    ("note".into(), "你好".into()),
                ],
            ),
        ];
        for req in cases {
            let msg = ClientMsg {
                session: s(),
                path: "/item/123".to_string(),
                request: req.clone(),
            };
            let bytes = encode_client(&msg);
            assert!(bytes.len() <= MAX_FRAME);
            assert_eq!(decode_client(&bytes), Some(msg), "case {:?}", req);
        }
    }

    #[test]
    fn server_roundtrip() {
        let entries = vec![
            ManifestEntry {
                id: 1,
                dynamic: false,
                hash: 42,
                title: "白塔".into(),
                path: "/".into(),
                matches: None,
            },
            ManifestEntry {
                id: 7,
                dynamic: true,
                hash: 0,
                title: String::new(),
                path: "/guestbook".into(),
                matches: Some("/item/".into()),
            },
        ];
        let msgs = vec![
            ServerMsg::Hello {
                version: VERSION,
                site: "home".into(),
            },
            ServerMsg::Resolved {
                name: "shop".into(),
                endpoint: "199.7.140.217:7002".into(),
                tls: true,
                title: "商店".into(),
            },
            ServerMsg::Manifest {
                session: s(),
                entries,
            },
            ServerMsg::Page {
                id: 2,
                total: 100,
                offset: 0,
                data: b"# hi".to_vec(),
            },
            ServerMsg::Image {
                id: 3,
                total: 9,
                offset: 4,
                data: vec![1, 2, 3, 4, 5],
            },
            ServerMsg::Empty(EmptyReason::Missing),
        ];
        for m in msgs {
            let bytes = encode_server(&m);
            assert!(bytes.len() <= MAX_FRAME);
            assert_eq!(decode_server(&bytes), Some(m));
        }
    }

    #[test]
    fn truncation_never_panics() {
        let msg = ClientMsg {
            session: s(),
            path: String::new(),
            request: Request::Submit(7, vec![("a".into(), "b".into())]),
        };
        let bytes = encode_client(&msg);
        for cut in 0..bytes.len() {
            let _ = decode_client(&bytes[..cut]);
        }
        let evil = vec![0xff; 64];
        for cut in 0..evil.len() {
            let _ = decode_client(&evil[..cut]);
            let _ = decode_server(&evil[..cut]);
        }
    }

    #[test]
    fn rejects_bad_lengths() {
        let msg = ClientMsg {
            session: s(),
            path: String::new(),
            request: Request::Manifest,
        };
        let mut bytes = encode_client(&msg);
        bytes[0] = 0xff;
        assert_eq!(decode_client(&bytes), None);

        let mut bytes = encode_client(&msg);
        bytes.push(0);
        assert_eq!(decode_client(&bytes), None);
    }
}
