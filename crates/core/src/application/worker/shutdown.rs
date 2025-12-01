// Worker Shutdown Token

use tokio::sync::watch;

/// Shutdown signal for graceful termination
#[derive(Clone)]
pub struct ShutdownToken {
    rx: watch::Receiver<bool>,
}

impl ShutdownToken {
    /// Check if shutdown was requested
    pub fn is_shutdown(&self) -> bool {
        *self.rx.borrow()
    }

    /// Wait for shutdown signal
    pub async fn wait(&mut self) {
        let _ = self.rx.changed().await;
    }
}

/// Shutdown sender
pub struct ShutdownSender {
    tx: watch::Sender<bool>,
}

impl ShutdownSender {
    /// Signal shutdown to all workers
    pub fn shutdown(&self) {
        let _ = self.tx.send(true);
    }
}

/// Create a shutdown channel
pub fn shutdown_channel() -> (ShutdownSender, ShutdownToken) {
    let (tx, rx) = watch::channel(false);
    (ShutdownSender { tx }, ShutdownToken { rx })
}
