//! Page fetching off the UI thread.
//!
//! Over a real network a page request costs a round trip; doing that on the UI
//! thread freezes the whole window (no repaint, no input, no "loading" hint).
//! The worker below owns the protocol client so the session stays consistent
//! between page requests and form submissions.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::net::{Client, Entry, Found};
use crate::routes::TlsMode;

pub enum Job {
    Manifest,
    Page { id: u16, path: String },
    Submit {
        id: u16,
        path: String,
        fields: Vec<(String, String)>,
    },
    /// Ask the server we are on where another site lives.
    Resolve(String),
}

pub enum Msg {
    /// `None` means the server did not answer.
    Manifest(Option<Vec<Entry>>),
    /// `None` means this page could not be fetched.
    Page { id: u16, text: Option<String> },
    /// `None` means the server does not know that name.
    Resolved {
        name: String,
        found: Option<Found>,
    },
}

pub struct Fetcher {
    tx: Sender<Job>,
    rx: Receiver<Msg>,
    in_flight: usize,
}

impl Fetcher {
    pub fn new(addr: String, tls: TlsMode) -> Self {
        let (job_tx, job_rx) = channel::<Job>();
        let (msg_tx, msg_rx) = channel::<Msg>();

        thread::spawn(move || {
            let mut client = Client::with_tls(addr, tls);
            let mut entries: HashMap<u16, Entry> = HashMap::new();
            let mut static_cache: HashMap<u16, (u64, String)> = HashMap::new();
            while let Ok(job) = job_rx.recv() {
                let msg = match job {
                    Job::Manifest => {
                        let manifest = client.manifest().ok();
                        if let Some(list) = &manifest {
                            entries.clear();
                            for entry in list {
                                entries.insert(entry.id, entry.clone());
                            }
                        }
                        Msg::Manifest(manifest)
                    }
                    Job::Page { id, path } => {
                        if let Some(entry) = entries.get(&id) {
                            if !entry.dynamic {
                                if let Some((hash, text)) = static_cache.get(&id) {
                                    if *hash == entry.hash {
                                        let _ = msg_tx.send(Msg::Page {
                                            id,
                                            text: Some(text.clone()),
                                        });
                                        continue;
                                    }
                                }
                            }
                        }
                        let text = client.page(id, &path).ok();
                        if let (Some(entry), Some(text)) = (entries.get(&id), &text) {
                            if !entry.dynamic {
                                static_cache.insert(id, (entry.hash, text.clone()));
                            }
                        }
                        Msg::Page { id, text }
                    }
                    Job::Submit { id, path, fields } => {
                        match client.submit(id, fields, &path) {
                            Ok((id, text)) => Msg::Page {
                                id,
                                text: Some(text),
                            },
                            Err(_) => Msg::Page { id, text: None },
                        }
                    }
                    Job::Resolve(name) => {
                        let found = client.resolve(&name).ok();
                        Msg::Resolved { name, found }
                    }
                };
                if msg_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        Self {
            tx: job_tx,
            rx: msg_rx,
            in_flight: 0,
        }
    }

    fn send(&mut self, job: Job) {
        if self.tx.send(job).is_ok() {
            self.in_flight += 1;
        }
    }

    pub fn manifest(&mut self) {
        self.send(Job::Manifest);
    }

    pub fn page(&mut self, id: u16, path: String) {
        self.send(Job::Page { id, path });
    }

    pub fn submit(&mut self, id: u16, path: String, fields: Vec<(String, String)>) {
        self.send(Job::Submit { id, path, fields });
    }

    pub fn resolve(&mut self, name: String) {
        self.send(Job::Resolve(name));
    }

    pub fn drain(&mut self) -> Vec<Msg> {
        let mut out = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            out.push(msg);
        }
        out
    }

    pub fn busy(&self) -> bool {
        self.in_flight > 0
    }
}
