# RakNet

A fully functional RakNet implementation in rust, asynchronously driven.

```rust
// Create a server
use raknet::Listener;
use raknet::util::handle;
use raknet::util::mcpe;

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
    server.motd = Motd::new(
        "Rust Bedrock Minecraft server",
        // 100 players, maximum
        100,
        // The minecraft version to display
        "1.18.0",
        // The Gamemode to display
        mcpe::Gamemode::Creative
    );

    // Begin listening to incoming connections
    server.start().await;

    loop {
        let (conn, stream, _) = server.accept().await.unwrap();

        // You can use the default handler, or create your own
        // the default handler
        tokio::spawn(handle(conn, stream));
        // your own handler
        tokio::spawn(my_handler(conn, stream));
    }
}
```