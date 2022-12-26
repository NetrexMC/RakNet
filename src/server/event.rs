use std::net::SocketAddr;

use crate::{connection::state::ConnectionState, protocol::mcpe::motd::Motd};

#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// A request to refresh the MOTD,
    /// the second value in this tuple represents
    /// the `Motd` that will be used if the event is
    /// disregarded.
    RefreshMotdRequest(SocketAddr, Motd),
    /// Requests the client to update their mtu size.
    /// This event is dispatched before the client fully connects
    /// allowing you to control the MtuSize.
    SetMtuSize(u16),
    /// Disconnect the client immediately
    /// Sent to the client to immediately disconnect the client.
    /// If you ignore this, the connection will be dropped automatically
    /// however this event is fired to allow graceful handling of a disconnect
    DisconnectImmediately,
    /// A request from the listener to update a connection's state.
    /// This is done during handshake or if the connection is timed out.
    UpdateConnectionState(ConnectionState),
}

#[derive(Debug, Clone)]
pub enum ServerEventResponse {
    /// The response to a `RefreshMotdRequest`.
    RefreshMotd(Motd),
    /// A generic response that acknowledges the event was recieved, but
    /// no actions were taken.
    ///
    /// VALID FOR ALL EVENTS
    Acknowledged,
}
