# rak-rs

A fully functional RakNet implementation in pure rust, asynchronously driven.

<a href="https://discord.gg/y4aWA5MQxK"><img src="https://img.shields.io/discord/846586369568800798.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2"></a>

## Getting Started

RakNet (rak-rs) is available on [crates.io](https://crates.io/crates/rak-rs), to use it, add the following to your `Cargo.toml`:

```toml
[dependencies]
rak-rs = "0.3.3"
```

## Features

This RakNet implementation comes with 3 primary features, `async_std`, `async_tokio` and `mcpe`.  However, by default, only `async_std` is enabled, and `mcpe` requires you to modify your `Cargo.toml`.

If you wish to use these features, add them to your `Cargo.toml` as seen below:

```toml
[dependencies]
rak-rs = { version = "0.3.3", default-features = false, features = [ "async_tokio", "mcpe" ] }
```



rak-rs also provides the following modules:

- [`rak_rs::client`](https://docs.rs/rak-rs/latest/rak-rs/client) - A client implementation of RakNet, allowing you to connect to a RakNet server.
- [`rak_rs::connection`](https://docs.rs/rak-rs/latest/rak-rs/client) - A bare-bones implementation of a Raknet peer, this is mainly used for types.
- [`rak_rs::error`](https://docs.rs/rak-rs/latest/rak-rs/error) - A module with errors that both the Client and Server can respond with.
- [`rak_rs::protocol`](https://docs.rs/rak-rs/latest/rak-rs/protocol) - A lower level implementation of RakNet, responsible for encoding and decoding packets.
- [`rak_rs::server`](https://docs.rs/rak-rs/latest/rak-rs/server) - The base server implementation of RakNet.
- [`rak_rs::util`](https://docs.rs/rak-rs/latest/rak-rs/utils)  - General utilities used within `rak-rs`.

# Client

The `client` module provides a way for you to interact with RakNet servers with code.

**Example:**

```rust
use rak_rs::client::{Client, DEFAULT_MTU};
use std::net::ToSocketAddrs;

#[async_std::main]
async fn main() {
    let version: u8 = 10;
    let mut client = Client::new(version, DEFAULT_MTU);

    client.connect("my_server.net:19132").await.unwrap();

    // receive packets
    loop {
        let packet = client.recv().await.unwrap();

        println!("Received a packet! {:?}", packet);

        client.send_ord(vec![254, 0, 1, 1], Some(1));
    }
}

```

# Server

A RakNet server implementation in pure rust.

**Example:**

```rust
use rakrs::connection::Connection;
use rakrs::Listener;
use rakrs::

#[async_std::main]
async fn main() {
    let mut server = Listener::bind("0.0.0.0:19132").await.unwrap();
    server.start().await.unwrap();

    loop {
        let conn = server.accept().await;
        async_std::task::spawn(handle(conn.unwrap()));
    }
}

async fn handle(mut conn: Connection) {
    loop {
        // keeping the connection alive
        if conn.is_closed() {
            println!("Connection closed!");
            break;
        }
        if let Ok(pk) = conn.recv().await {
            println!("Got a connection packet {:?} ", pk);
        }
    }
}
```
