use binary_utils::*;
use std::{collections::VecDeque, sync::Arc, time::SystemTime};

use crate::{
    internal::{
        frame::reliability::Reliability,
        queue::{Queue, SendPriority},
        RakConnHandler, RakConnHandlerMeta,
    },
    protocol::{mcpe::motd::Motd, online::Disconnect, Packet},
    rak_debug,
    server::{RakEvent, RakNetVersion},
};

use crate::protocol::handler::{handle_offline, handle_online};

use super::state::ConnectionState;

pub type SendCommand = (String, Vec<u8>);

#[derive(Debug, Clone)]
pub struct Connection {
    /// The tokenized address of the connection.
    /// This is the identifier rak-rs will use to identify the connection.
    /// It follows the format `<ip>:<port>`.
    pub address: String,
    /// The current state of the connection.
    /// This is used to determine what packets can be sent and at what times.
    /// Some states are used internally to rak-rs, but are not used in actual protocol
    /// such as "Unidentified" and "Online".
    pub state: ConnectionState,
    /// The maximum transfer unit for the connection.
    /// Any outbound packets will be sharded into frames of this size.
    /// By default minecraft will use `1400` bytes. However raknet has 16 bytes of overhead.
    /// so this may be reduced as `1400 - 16` which is `1384`.
    pub mtu: u16,
    /// The last recieved time.
    /// This is used to determine if the connection has timed out.
    /// This is the time the last packet was recieved.
    pub recv_time: SystemTime,
    /// The time the server started.
    /// Used in pings
    pub start_time: SystemTime,
    /// The RakNet Version of the server.
    /// This is used to determine if the player can reliably join the server.
    pub raknet_version: RakNetVersion,
    /// Minecraft specific, the message of the day.
    pub motd: Motd,
    /// A reference to the server id.
    pub server_guid: u64,
    /// The packet queue for the connection.
    /// This is used to store packets that need to be sent, any packet here **WILL** be batched!
    pub queue: Queue<Vec<u8>>,
    /// This is an internal channel used on the raknet side to send packets to the user immediately.
    /// DO NOT USE THIS!
    pub send_channel: Arc<tokio::sync::mpsc::Sender<SendCommand>>,
    /// This is internal! This is used to dispatch events to the user.
    /// This will probably change in the near future, however this will stay,
    /// until that happens.
    pub event_dispatch: VecDeque<RakEvent>,
    /// This is internal! This is used to handle all raknet packets, like frame, ping etc.
    pub(crate) rakhandler: RakConnHandlerMeta,
    /// This is internal! This is used to remove the connection if something goes wrong with connection states.
    /// (which is likely)
    ensure_disconnect: bool,
}

impl Connection {
    pub fn new(
        address: String,
        send_channel: Arc<tokio::sync::mpsc::Sender<SendCommand>>,
        start_time: SystemTime,
        server_guid: u64,
        port: String,
        raknet_version: RakNetVersion,
    ) -> Self {
        Self {
            address,
            state: ConnectionState::Unidentified,
            mtu: 1400,
            recv_time: SystemTime::now(),
            start_time,
            motd: Motd::new(server_guid, port),
            server_guid,
            queue: Queue::new(),
            send_channel,
            event_dispatch: VecDeque::new(),
            raknet_version,
            ensure_disconnect: false,
            rakhandler: RakConnHandlerMeta::new(),
        }
    }

    /// Get the maximum allowed size of a entire frame packet.
    /// This is the MTU - the size of all possible raknet headers,
    /// so: `40 (Datagram Protocol) + 20 (Raknet)`
    pub fn max_frame_size(&self) -> usize {
        self.mtu as usize - 60
    }

    /// Adds the given stream to the connection's queue by priority.
    /// If instant is set to "true" the packet will be sent immediately.
    pub fn send(&mut self, stream: Vec<u8>, instant: bool) {
        if instant {
            // We're not going to batch this packet, so send it immediately.
            self.send_immediate(stream);
        } else {
            // We're going to batch this packet, so push it to the queue.
            self.queue.push(stream, SendPriority::Normal);
        }
    }

    /// This method should be used externally to send packets to the connection.
    /// Packets here will be batched together and sent in frames.
    pub fn send_stream(&mut self, stream: Vec<u8>, priority: SendPriority) {
        if priority == SendPriority::Immediate {
            RakConnHandler::send_framed(self, stream, Reliability::ReliableOrd);
        } else {
            self.queue.push(stream, priority);
        }
    }

    /// Immediately send the packet to the connection.
    /// This will not automatically batch the packet.
    pub fn send_immediate(&mut self, stream: Vec<u8>) {
        // check the context
        if let Ok(_) =
            futures_executor::block_on(self.send_channel.send((self.address.clone(), stream)))
        {
            // GREAT!
        } else {
            rak_debug!("Failed to send packet to {}", self.address);
        }
    }

    /// Sends the packet inside a frame and may queue it based on priority.
    /// All sent packets will be sent in reliably ordered frames.
    ///
    /// WARNING: DO NOT USE THIS FOR PACKETS THAT EXCEED MTU SIZE!
    pub fn send_frame(&mut self, stream: Vec<u8>, priority: SendPriority) {
        if priority == SendPriority::Immediate {
            // we need to batch this frame immediately.
            RakConnHandler::send_framed(self, stream, Reliability::ReliableOrd);
        } else {
            // we need to batch this frame.
            self.queue.push(stream, priority);
        }
    }

    /// This will send a raknet packet to the connection.
    /// This method will automatically parse the packet and send it by the given priority.
    pub fn send_packet(&mut self, packet: Packet, priority: SendPriority) {
        // we can check the kind, if it's an online packet we need to frame it.
        if packet.is_online() {
            self.send_frame(packet.parse().unwrap(), priority);
            return;
        }

        if priority == SendPriority::Immediate {
            self.send_immediate(packet.parse().unwrap());
        } else {
            self.queue
                .push(packet.parse().unwrap(), SendPriority::Normal);
        }
    }

    pub fn recv(&mut self, payload: &Vec<u8>) {
        self.recv_time = SystemTime::now();

        // build the packet
        if let Ok(packet) = Packet::compose(&payload, &mut 0) {
            // the packet is internal, let's check if it's an online packet or offline packet
            // and handle it accordingly.
            if packet.is_online() {
                // we recieved a frame packet out of scope.
                // return an error.
                return;
            } else {
                // offline packet
                // handle the disconnected packet
                handle_offline(self, packet);

                // let's verify our state.
                if !self.state.is_reliable() {
                    // we got a packet when the client state was un-reliable, we're going to force the client
                    // to un-identified.
                    self.state = ConnectionState::Unidentified;
                }
            }
        } else {
            // this packet could be a Ack or Frame
            // lets pass it to the rak handler. The rakhandler will invoke `connection.handle` which is
            // where we handle the online packets.
            if let Err(e) = RakConnHandler::handle(self, payload) {
                rak_debug!(
                    "We got a packet that we couldn't parse! Probably a Nak or Frame! Error: {}",
                    e
                );
            }

            // let's update the client state to connected.
            if !self.state.is_reliable() {
                self.state = ConnectionState::Connected;
            }
        }
    }

    /// This is called by the rak handler when each frame is decoded.
    /// These packets are usually online packets or game packets!
    pub(crate) fn handle(&mut self, buffer: Vec<u8>) {
        // check if the payload is a online packet.
        if let Ok(packet) = Packet::compose(&buffer, &mut 0) {
            // this is a packet! let's check the variety.
            if packet.is_online() {
                // online packet
                // handle the online packet
                if let Err(_) = handle_online(self, packet.clone()) {
                    // unknown packet lol
                    rak_debug!("Unknown packet! {:#?}", packet);
                }
            } else {
                // offline packet,
                // we can't handle offline packets sent within a frame.
                // we need to handle them in the `connection.recv` method.
                // we're going to force the client to be disconnected as this is not a valid packet.
                self.disconnect("Incorrect protocol usage within raknet.", true);
            }
        } else {
            // this isn't an online packet we know about, so we're going to emit an event here.
            // this is probably a game packet.
            self.event_dispatch
                .push_back(RakEvent::GamePacket(self.address.clone(), buffer));
        }
    }

    pub fn disconnect<S: Into<String>>(&mut self, reason: S, server_initiated: bool) {
        // disconnect!!!
        self.event_dispatch
            .push_back(RakEvent::Disconnect(self.address.clone(), reason.into()));
        // actually handle this internally, cause we can't send packets if we're disconnected.
        self.state = ConnectionState::Offline;
        // the following is a hack to make sure the connection is removed from the server.
        self.ensure_disconnect = true;
        // We also need to flush the queue so packets aren't sent, because they are now useless.
        self.queue.flush();
        // Freeze the queue, just in case this is a server sided disconnect.
        // Otherwise this is useless.
        self.queue.frozen = true;

        if server_initiated {
            self.send_packet(Disconnect {}.into(), SendPriority::Immediate);
        }
    }

    /// This reads an internal value! This may not be in relation to the client's CURRENT state!
    pub fn is_disconnected(&self) -> bool {
        return self.ensure_disconnect == true;
    }

    /// This is called every RakNet tick.
    /// This is used to update the connection state and send `Priority::Normal` packets.
    /// as well as other internal stuff like updating flushing Ack and Nack.
    pub fn tick(&mut self) {
        if self.state.is_reliable() {
            // we need to update the state of the connection.
            // check whether or not we're becoming un-reliable.
            if self.recv_time.elapsed().unwrap().as_secs() > 8 {
                // we're becoming un-reliable.
                self.state = ConnectionState::TimingOut;
            }
            // tick the rakhandler
            RakConnHandler::tick(self);
        } else {
            if self.recv_time.elapsed().unwrap().as_secs() >= 15 {
                // we're not reliable anymore.
                self.state = ConnectionState::Disconnected;
                self.disconnect("Timed Out", true);
                return;
            }
        }
    }
}
