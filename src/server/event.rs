use std::net::SocketAddr;

use crate::protocol::mcpe::motd::Motd;

#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// A request to refresh the MOTD,
    /// the second value in this tuple represents
    /// the `Motd` that will be used if the event is
    /// disregarded.
    RefreshMotdRequest(SocketAddr, Motd),
    /// The response to a `RefreshMotdRequest`.
    RefreshMotd(Motd),

}