//! Scripted SSE, written the way B2 and B3 write theirs (ADR 1.10.26x D5).
//!
//! B2's event stream carries `event:` names, `id:` lines and heartbeats as
//! comments; B3's dialogue turn carries `delta` / `notice` / `done` / `error`
//! frames with no `id:`. The server writes exactly the frames a case scripts,
//! and produces its faults: a dropped connection after N events, a silent
//! gap after the frames, and fragmented writes.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;

use crate::case::{Fault, Frame};

pub type Chunk = Result<Bytes, std::io::Error>;

/// One frame's bytes, terminated by the blank line.
pub fn encode(frame: &Frame) -> String {
    if let Some(comment) = &frame.comment {
        return format!(": {comment}\n\n");
    }
    let mut out = String::new();
    if let Some(event) = &frame.event {
        out.push_str(&format!("event: {event}\n"));
    }
    if let Some(id) = &frame.id {
        out.push_str(&format!("id: {id}\n"));
    }
    let data = frame.data.as_ref().map(|d| d.to_string()).unwrap_or_default();
    out.push_str(&format!("data: {data}\n\n"));
    out
}

/// The body stream for `frames` under `fault`. A drop ends the stream with
/// an error, which aborts the connection mid-body. `open` counts it until
/// the server drops the body: the connection is gone (K1 D12, EK9).
pub fn stream(frames: Vec<Frame>, fault: Fault, open: Arc<AtomicUsize>) -> Counted {
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(write(frames, fault, tx));
    open.fetch_add(1, Ordering::SeqCst);
    Counted { inner: ReceiverStream::new(rx), open }
}

/// A body stream that is counted while it is connected.
pub struct Counted {
    inner: ReceiverStream<Chunk>,
    open: Arc<AtomicUsize>,
}

impl Stream for Counted {
    type Item = Chunk;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Chunk>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.open.fetch_sub(1, Ordering::SeqCst);
    }
}

async fn write(frames: Vec<Frame>, fault: Fault, tx: mpsc::Sender<Chunk>) {
    let mut events = 0;
    for frame in &frames {
        if fault.drop_after_events.is_some_and(|n| events >= n) {
            return drop_now(&tx).await;
        }
        if frame.comment.is_none() {
            events += 1;
        }
        let text = encode(frame);
        if !send(&tx, text, fault.delay_fragment_ms).await {
            return;
        }
    }
    if fault.drop_after_events.is_some_and(|n| events >= n) {
        return drop_now(&tx).await;
    }
    if let Some(gap) = fault.heartbeat_gap_s {
        tokio::time::sleep(Duration::from_secs_f64(gap)).await;
    }
}

/// Aborts the connection. The pause first lets the frames already written
/// reach the wire: an error polled beside them would discard them unsent.
async fn drop_now(tx: &mpsc::Sender<Chunk>) {
    tokio::time::sleep(Duration::from_millis(50)).await;
    let _ = tx.send(Err(std::io::Error::other("scripted drop"))).await;
}

/// Writes one frame, in two halves when fragmenting. False once the client is gone.
async fn send(tx: &mpsc::Sender<Chunk>, text: String, fragment_ms: Option<u64>) -> bool {
    let Some(ms) = fragment_ms else {
        return tx.send(Ok(Bytes::from(text))).await.is_ok();
    };
    let split = text.len() / 2;
    let (a, b) = (text[..split].to_string(), text[split..].to_string());
    if tx.send(Ok(Bytes::from(a))).await.is_err() {
        return false;
    }
    tokio::time::sleep(Duration::from_millis(ms)).await;
    tx.send(Ok(Bytes::from(b))).await.is_ok()
}
