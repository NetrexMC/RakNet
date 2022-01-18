use super::state::ConnectionState;

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
}