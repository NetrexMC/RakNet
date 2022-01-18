use std::net::{SocketAddr, ToSocketAddrs};

pub fn to_address_token(remote: SocketAddr) -> String {
    let mut address = remote.ip().to_string();
    address.push_str(":");
    address.push_str(remote.port().to_string().as_str());
    return address;
}

pub fn from_address_token(remote: String) -> SocketAddr {
    let mut parsed = remote
        .to_socket_addrs()
        .expect("Could not parse remote address.");
    SocketAddr::from(parsed.next().unwrap())
}