//! Page fetching off the UI thread.
//!
//! Over a real network a page request costs a round trip; doing that on the UI
//! thread freezes the whole window (no repaint, no input, no "loading" hint).
//! The worker below owns the protocol client so the session stays consistent
//! between page requests and form submissions.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::net::{Client, Entry};

pub enum Job {
    Manifest,
    Page(u16),
    Submit(u16, Vec<(String, String)>),
}

pub enum Msg {
    /// `None` means the server did not answer.
    Manifest(Option<Vec<Entry>>),
    /// `None` means this page could not be fetched.
    Page { id: u16, text: Option<String> },
}

pub struct Fetcher {
    tx: Sender<Job>,
    rx: Receiver<Msg>,
    in_flight: usize,
}

impl Fetcher {
    pub fn new(addr: String) -> Self {
        let (job_tx, job_rx) = channel::<Job>();
        let (msg_tx, msg_rx) = channel::<Msg>();

        thread::spawn(move || {
            let mut client = Client::new(addr);
            while let Ok(job) = job_rx.recv() {
                let msg = match job {
                    Job::Manifest => Msg::Manifest(client.manifest().ok()),
                    Job::Page(id) => Msg::Page {
                        id,
                        text: client.page(id).ok(),
                    },
                    Job::Submit(id, fields) => match client.submit(id, fields) {
                        Ok((id, text)) => Msg::Page {
                            id,
                            text: Some(text),
                        },
                        Err(_) => Msg::Page { id, text: None },
                    },
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

    pub fn page(&mut self, id: u16) {
        self.send(Job::Page(id));
    }

    pub fn submit(&mut self, id: u16, fields: Vec<(String, String)>) {
        self.send(Job::Submit(id, fields));
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
