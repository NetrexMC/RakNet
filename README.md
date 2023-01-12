# RakNet

A fully functional RakNet implementation in rust, asynchronously driven.

### Installation

By default `rakrs` will use `async_std` feature, which in turn, utilizes the `async_std` crate. To use `tokio` you will need to add it to cargo features as `async_tokio`.

Using the tokio library with the `mcpe` feature may look like the following:

```toml
rakrs = { version = "0.3.0", features = [ "mcpe", "async_std" ], default-features = false }
```

```rust
// Create a server
use rakrs::Listener;
use rakrs::util::handle;
use rakrs::util::mcpe;

async fn my_handler(conn: RakConnection, mut stream: RakStream) {
    // The `conn.recv()` method constructs a `Packet` from the stream
    // Which becomes usable later.
    while let Some(packet) = conn.recv(&mut stream).await {
        if let Ok(packet) = packet {
            // RakNet packets are sent here! We can send some back as well!
            let hello = raknet::protocol::online::ConnectedPing::new();
            conn.send(hello.into(), Reliability::Reliable).await;
        }
    }
}

async fn main() {
    // Bind to a socket and allow minecraft protocol
    let mut server = Listener::bind("0.0.0.0:19132", true).await;
    server.motd.name = "Rust Bedrock Minecraft server";
    server.motd.player_count = 100;
    server.motd.player_max = 200;
    server.motd.gamemode = mcpe::Gamemode::Survival;

    // Begin listening to incoming connections
    server.start().await;

    loop {
        let conn = server.accept().await.unwrap();

        // You can use the default handler, or create your own
        // the default handler
        tokio::spawn(handle(conn, stream));
        // your own handler
        tokio::spawn(my_handler(conn, stream));
    }
}
```