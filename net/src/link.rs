//! A client connection pumped on its own thread.
//!
//! Renet measures round-trip time from packet acknowledgements, and a packet is
//! only acknowledged when the transport next updates. Driving the transport from
//! the render loop would add a whole frame to every exchange, so a slow or
//! stalled frame would read as network latency and delay the server's inputs.
//! This thread reads the socket at a steady rate whatever the frame rate.
use crate::transport::ClientTransport;
use renet::RenetClient;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::{Duration, Instant},
};

/// How often the network thread reads the socket.
const PUMP_INTERVAL: Duration = Duration::from_millis(1);
/// Longest the network thread goes without sending, so acknowledgements and
/// reliable resends keep flowing while the game thread is stalled.
const SEND_INTERVAL: Duration = Duration::from_millis(25);

pub(crate) struct Link {
    pub net: RenetClient,
    pub transport: ClientTransport,
    /// The first transport failure; the connection is finished after it.
    pub error: Option<String>,
    /// When the connection was last advanced, by either thread.
    last: Instant,
    /// When packets were last sent, by either thread.
    sent: Instant,
}

impl Link {
    /// Advance the connection to now and read what has arrived. Sends when a
    /// packet just arrived (so its acknowledgement leaves at once and
    /// round-trip times stay true), or after `SEND_INTERVAL` without a send.
    ///
    /// Renet attaches pending acknowledgements to every send, so sending on
    /// every pass would emit a packet each millisecond and flood the server.
    pub fn service(&mut self) {
        if self.error.is_some() {
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last);
        self.last = now;
        self.net.update(dt);
        if let Err(e) = self.transport.update(dt, &mut self.net) {
            self.error = Some(e);
            return;
        }
        let arrived = self.transport.time_since_last_received_packet() < dt.max(PUMP_INTERVAL);
        if arrived || now.duration_since(self.sent) >= SEND_INTERVAL {
            let _ = self.send();
        }
    }

    /// Send queued messages and pending acknowledgements now.
    pub fn send(&mut self) -> Result<(), String> {
        self.sent = Instant::now();
        let result = self.transport.send_packets(&mut self.net);
        if let Err(e) = &result {
            self.error.get_or_insert_with(|| e.clone());
        }
        result
    }
}

pub(crate) struct LinkHandle {
    shared: Arc<Mutex<Link>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl LinkHandle {
    pub fn spawn(net: RenetClient, transport: ClientTransport) -> Result<Self, String> {
        let now = Instant::now();
        let shared = Arc::new(Mutex::new(Link {
            net,
            transport,
            error: None,
            last: now,
            sent: now,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let shared = shared.clone();
            let stop = stop.clone();
            thread::Builder::new()
                .name("racecraft-net".into())
                .spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        lock(&shared).service();
                        thread::sleep(PUMP_INTERVAL);
                    }
                })
                .map_err(|e| e.to_string())?
        };
        Ok(Self {
            shared,
            stop,
            worker: Some(worker),
        })
    }

    /// Exclusive access to the connection, shared with the network thread.
    pub fn lock(&self) -> MutexGuard<'_, Link> {
        lock(&self.shared)
    }
}

/// A panic elsewhere leaves the connection itself consistent; keep using it.
fn lock(shared: &Mutex<Link>) -> MutexGuard<'_, Link> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Drop for LinkHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
