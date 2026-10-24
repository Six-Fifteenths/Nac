//! Background image loader: fetching and decoding must not stall the UI thread.

use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::net::Client;
use crate::routes::TlsMode;

pub struct Loaded {
    pub id: u16,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub enum Msg {
    Done(Loaded),
    Failed(u16),
}

pub struct Loader {
    tx: Sender<u16>,
    rx: Receiver<Msg>,
    pub pending: HashSet<u16>,
    pub failed: HashSet<u16>,
}

impl Loader {
    pub fn new(addr: String, tls: TlsMode) -> Self {
        let (req_tx, req_rx) = channel::<u16>();
        let (msg_tx, msg_rx) = channel::<Msg>();

        thread::spawn(move || {
            let mut client = Client::with_tls(addr, tls);
            while let Ok(id) = req_rx.recv() {
                let msg = match client.image(id) {
                    Ok(bytes) => match image::load_from_memory(&bytes) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            Msg::Done(Loaded {
                                id,
                                width: rgba.width(),
                                height: rgba.height(),
                                rgba: rgba.into_raw(),
                            })
                        }
                        Err(_) => Msg::Failed(id),
                    },
                    Err(_) => Msg::Failed(id),
                };
                if msg_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        Self {
            tx: req_tx,
            rx: msg_rx,
            pending: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    pub fn request(&mut self, id: u16) {
        if self.pending.contains(&id) || self.failed.contains(&id) {
            return;
        }
        self.pending.insert(id);
        let _ = self.tx.send(id);
    }

    pub fn drain(&mut self) -> Vec<Msg> {
        let mut out = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            match &msg {
                Msg::Done(loaded) => {
                    self.pending.remove(&loaded.id);
                }
                Msg::Failed(id) => {
                    self.pending.remove(id);
                    self.failed.insert(*id);
                }
            }
            out.push(msg);
        }
        out
    }

    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
    }
}
