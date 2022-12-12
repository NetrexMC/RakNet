use std::net::SocketAddr;

use crate::protocol::mcpe::motd::Motd;

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
}

#[derive(Debug, Clone)]
pub enum ServerEventResponse {
    /// The response to a `RefreshMotdRequest`.
    RefreshMotd(Motd),
}
