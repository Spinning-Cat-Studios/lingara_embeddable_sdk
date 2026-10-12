//! A few-line event-stream peer (D12): every request is answered `200
//! text/event-stream`, and the connection is then held open and never
//! written to again. `closed()` reports when the client hung up.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct Loopback {
    pub url: String,
    opened: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
}

impl Loopback {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}", listener.local_addr().expect("addr"));
        let (opened, closed) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let (o, c) = (Arc::clone(&opened), Arc::clone(&closed));
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (o, c) = (Arc::clone(&o), Arc::clone(&c));
                std::thread::spawn(move || hold(stream, &o, &c));
            }
        });
        Self { url, opened, closed }
    }

    pub fn opened(&self) -> usize {
        self.opened.load(Ordering::SeqCst)
    }

    pub fn closed(&self) -> usize {
        self.closed.load(Ordering::SeqCst)
    }
}

/// Reads the request head, answers the stream's headers, then waits for EOF.
fn hold(mut stream: TcpStream, opened: &AtomicUsize, closed: &AtomicUsize) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
        head.push(byte[0]);
    }
    let answer = b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\n\r\n";
    if stream.write_all(answer).is_err() {
        return;
    }
    opened.fetch_add(1, Ordering::SeqCst);
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    let mut sink = [0u8; 256];
    while stream.read(&mut sink).is_ok_and(|n| n > 0) {}
    closed.fetch_add(1, Ordering::SeqCst);
}
