use async_std::channel::{Receiver, Sender};

/// Notify is a struct that wraps a buffer channel
/// these channels are used to send messages to the main thread.
#[derive(Clone)]
pub struct Notify(pub Option<Sender<()>>, pub Receiver<()>);

impl Notify {
    /// Creates a new Notify struct.
    pub fn new() -> Self {
        let (send, recv) = async_std::channel::bounded(1);
        Self(Some(send), recv)
    }

    /// Sends a message to all listeners.
    pub async fn notify(&self) -> bool {
        if let Some(sender) = &self.0 {
            sender.close();
            true
        } else {
            false
        }
    }

    /// Waits for a message from the main thread.
    pub async fn wait(&self) -> bool {
        if let Err(_) = self.1.recv().await {
            false
        } else {
            true
        }
    }
}
