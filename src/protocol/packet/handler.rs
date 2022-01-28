use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::SystemTime;

use crate::connection::state::ConnectionState;
use crate::internal::queue::SendPriority;
use crate::internal::util::from_address_token;
use crate::protocol::util::Magic;
use crate::rak_debug;
use crate::{connection::Connection, server::RakEvent};

use super::offline::{IncompatibleProtocolVersion, OpenConnectReply, SessionInfoReply};
use super::online::{ConnectedPong, ConnectionAccept, OnlinePacket};
use super::OfflinePacket;
use super::{offline::UnconnectedPong, Packet};

/// The offline packet handler, responsible for handling
/// Ping, Pong, and other packets.
pub fn handle_offline(connection: &mut Connection, packet: Packet) {
    // check if the type of packet, we'll use a match statement
    let result = match packet.get_offline() {
        OfflinePacket::UnconnectedPing(_) => {
            // if the packet is a ping, we'll send a pong
            // and dispatch an event to update the Motd.
            connection.event_dispatch.push_back(RakEvent::Motd(
                connection.address.clone(),
                connection.motd.clone(),
            ));

            // send the pong to the server, and parse it!
            // we could compensate for decoding time, but there isn't
            // too much overhead there, so we'll just send as is.
            let pong = UnconnectedPong {
                server_id: connection.server_guid,
                timestamp: connection.start_time.elapsed().unwrap().as_millis() as u64,
                magic: Magic::new(),
                #[cfg(feature = "mcpe")]
                motd: connection.motd.clone(),
            };
            connection.send_packet(pong.into(), SendPriority::Immediate);
            Ok(())
        }
        OfflinePacket::OpenConnectRequest(pk) => {
            if pk.protocol != connection.raknet_version.to_u8() {
                let incompatible = IncompatibleProtocolVersion {
                    protocol: pk.protocol,
                    magic: Magic::new(),
                    server_id: connection.server_guid,
                };
                connection.send_packet(incompatible.into(), SendPriority::Immediate);
            }

            // The version is valid, we can send the reply.
            let reply = OpenConnectReply {
                server_id: connection.server_guid,
                // todo: Make this optional
                security: false,
                magic: Magic::new(),
                mtu_size: pk.mtu_size,
            };

            // we can actually save the requested mtu size from the client
            connection.mtu = pk.mtu_size;
            connection.send_packet(reply.into(), SendPriority::Immediate);
            Ok(())
        }
        OfflinePacket::SessionInfoRequest(pk) => {
            // todo: Actually check if we want the client to join the server!
            // todo: And disconnect them if we don't!
            let reply = SessionInfoReply {
                server_id: connection.server_guid,
                client_address: from_address_token(connection.address.clone()),
                magic: Magic::new(),
                mtu_size: pk.mtu_size,
                // todo: Again, make this optional
                security: false,
            };
            // the client is now officially in the "Connecting State"
            // let's validate the mtu
            if pk.mtu_size != connection.mtu {
                connection.mtu = pk.mtu_size;
                #[cfg(feature = "dbg")]
                rak_debug!(
                    "[RakNet] [{}] Recieved two different MTU sizes, setting to {}",
                    connection.address,
                    connection.mtu
                );
            }

            // the client is actually trying to connect.
            connection.state = ConnectionState::Connecting;
            connection.send_packet(reply.into(), SendPriority::Immediate);
            Ok(())
        }
        _ => {
            Err("A client can not send this packet, or the packet is not implemented for offline!")
        }
    };

    if let Err(e) = result {
        // we're not going to panic because that would be bad in prod, so we'll just log it.
        rak_debug!(
            "[RakNet] [{}] Received an offline packet that is not! {:?}",
            connection.address,
            e
        );
    };
}

pub fn handle_online(connection: &mut Connection, packet: Packet) -> Result<(), &str> {
    match packet.get_online() {
        OnlinePacket::ConnectedPing(pk) => {
            let response = ConnectedPong {
                ping_time: pk.time,
                pong_time: SystemTime::now()
                    .duration_since(connection.start_time)
                    .unwrap()
                    .as_millis() as i64,
            };
            connection.send_packet(response.into(), SendPriority::Immediate);
            Ok(())
        }
        OnlinePacket::ConnectionRequest(pk) => {
            let response = ConnectionAccept {
                system_index: 0,
                client_address: from_address_token(connection.address.clone()),
                internal_id: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 19132),
                request_time: pk.time,
                timestamp: SystemTime::now()
                    .duration_since(connection.start_time)
                    .unwrap()
                    .as_millis() as i64,
            };
            connection.send_packet(response.into(), SendPriority::Immediate);
            Ok(())
        }
        OnlinePacket::Disconnect(_) => {
            // Disconnect the client immediately.
            connection.disconnect("Client disconnected.", false);
            Ok(())
        }
        OnlinePacket::NewConnection(_) => {
            connection.state = ConnectionState::Connected;
            Ok(())
        }
        _ => Err("A client can not send this packet, or the packet is not implemented for online!"),
    }
}
