use std::collections::HashMap;

use crate::server::current_epoch;

#[derive(Debug, Clone)]
pub struct ReliableWindow {
    // The current window start and end
    window: (u32, u32),
    // The current window size
    size: u32,
    // the current queue of packets by timestamp
    queue: HashMap<u32, u64>,
}

impl ReliableWindow {
    pub fn new() -> Self {
        Self {
            window: (0, 2048),
            size: 2048,
            queue: HashMap::new(),
        }
    }

    pub fn insert(&mut self, index: u32) -> bool {
        // We already got this packet
        if index < self.window.0 || index > self.window.1 || self.queue.contains_key(&index) {
            return false;
        }

        self.queue.insert(index, current_epoch());

        // we need to update the window to check if the is within it.
        if index == self.window.0 {
            self.adjust();
        }

        return true;
    }

    /// Attempts to adjust the window size, removing all out of date packets
    /// from the queue.
    pub fn adjust(&mut self) {
        // remove all packets that are out of date, that we got before the window,
        // increasing the window start and end if we can.
        while self.queue.contains_key(&self.window.0) {
            self.queue.remove(&self.window.0);
            self.window.0 = self.window.0.wrapping_add(1);
            self.window.1 = self.window.1.wrapping_add(1);
        }

        // if the window is too small or too big, make sure it's the right size.
        // corresponding to self.size
        let curr_size = self.window.1.wrapping_sub(self.window.0);
        if curr_size < self.size {
            self.window.1 = self.window.0.wrapping_add(self.size);
        } else if curr_size > self.size {
            self.window.0 = self.window.1.wrapping_sub(self.size);
        }
    }

    /// Returns all the packets that are in the window.
    pub fn missing(&self) -> Vec<u32> {
        let mut missing = Vec::new();

        for i in self.window.0..self.window.1 {
            if !self.queue.contains_key(&i) {
                missing.push(i);
            }
        }

        missing
    }

    pub fn range(&self) -> (u32, u32) {
        self.window
    }

    /// Forcefully clears packets that are not in the window.
    /// This is used when the window is too small to fit all the packets.
    pub fn clear_outdated(&mut self) {
        //todo actually clear packets that are out of date!
        self.queue
            .retain(|k, _| *k >= self.window.0 && *k <= self.window.1);
    }
}

pub struct Window {
    // last round trip time
    pub rtt: u32,
}
