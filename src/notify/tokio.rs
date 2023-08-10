use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;

/// Notify is a struct that wraps a buffer channel
/// these channels are used to send messages to the main thread.
pub struct Notify(RwLock<Option<Sender<()>>>, RwLock<Option<Receiver<()>>>);

impl Notify {
    /// Creates a new Notify struct.
    pub fn new() -> Self {
        let (send, recv) = tokio::sync::mpsc::channel(1);
        Self(RwLock::new(Some(send)), RwLock::new(Some(recv)))
    }

    /// Sends a message to all listeners.
    pub async fn notify(&self) -> bool {
        let mut sender = self.0.write().await;
        if let Some(_) = sender.take() {
            true
        } else {
            false
        }
    }

    /// Waits for a message from the main thread.
    pub async fn wait(&self) -> bool {
        let mut receiver = self.1.write().await;
        if let Some(receiver) = receiver.as_mut() {
            receiver.recv().await;
            true
        } else {
            false
        }
    }
}
