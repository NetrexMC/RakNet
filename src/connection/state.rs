/// Connection States
/// These are all possible states of a raknet session, and while accessible externally
/// Please note that these are not states relied on within the original implementation of
/// raknet, which preserve both "Unconnected" and "Connected"
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum ConnectionState {
    /// The Session is not yet connected, but is actively trying to connect.
    /// Clients in this state are considered to be actively trying to connect.
    Connecting,

    /// The Session is connected and ready to send and receive packets.
    /// This is the state after a connection has been established.
    ///
    /// This state is applied once the `ConnectionHandshake` has been completed.
    Connected,

    /// The session is being timed out because it has not sent a packet in a while.
    /// The interval for this can be set in the Session Options.
    TimingOut,

    /// The session has been disconnected but is still in the process of cleaning up.
    /// This is the state after a disconnect has been requested, but the client still wants
    /// to send packets until its done.
    Disconnecting,

    /// The session has been disconnected and is ready to be removed.
    /// This is the state after a disconnect has been requested and the client has
    /// This is almost never used.
    Disconnected,

    /// The session is replying to the server but is not actually connected. This is
    /// the state where ping and pong packets are being sent. Similarly, this is
    /// the "Unconnected" state, hence "UnconnectedPing"
    Unidentified,

    /// The session has been identified and is ready to be connected.
    /// This is the state after a connection has been established.
    Identified,

    /// The session is not connected and is not trying to connect.
    /// During this state the session will be dropped. This state occurs when a client
    /// has completely stopped responding to packets or their socket is destroyed.
    /// This is not the same as the [`Disconnected`] state.
    ///
    /// [`Disconnected`]: crate::connection::state::ConnectionState::Disconnected
    Offline,
}

impl ConnectionState {
    /// Returns whether or not the Session is reliable.
    /// Reliable sessions are sessions that are not:
    /// - Offline
    /// - Disconnected
    /// - TimingOut
    pub fn is_reliable(&self) -> bool {
        match self {
            Self::Disconnected | Self::TimingOut | Self::Offline => false,
            _ => true,
        }
    }

    /// Returns whether or not the Session is available to recieve
    /// packets. Sessions in this state are:
    /// - Connected
    /// - Connecting
    /// - Unidentified
    /// - Disconnecting
    pub fn is_available(&self) -> bool {
        match self {
            Self::Connected | Self::Connecting | Self::Unidentified | Self::Disconnecting => true,
            _ => false,
        }
    }

    /// Returns whether or not the Session is in any "connected" state.
    /// Sessions in this state are:
    /// - Connected
    /// - Connecting
    pub fn is_connected(&self) -> bool {
        match self {
            Self::Connected | Self::Connecting => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::TimingOut => write!(f, "TimingOut"),
            Self::Disconnecting => write!(f, "Disconnecting"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Unidentified => write!(f, "Unidentified"),
            Self::Identified => write!(f, "Identified"),
            Self::Offline => write!(f, "Offline"),
        }
    }
}
