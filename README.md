# RakNet

A fully functional RakNet implementation in rust, asynchronously driven.

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

#### Thanks to:

- [@b24r0/rust-raknet](https://github.com/b23r0/rust-raknet)! I used this as a reference for `v3` which gave me a deeper understanding of async design patterns in rust! More specifically:
  
  - `Sephamore`. These are used throughout to help notify server of various signals, mainly `Close`. Sephamore's help by waiting for vairious tasks to shutdown before fully closing the listener.
  
  - `Notify`. These are simple by design, but they are used to synchronize actions across various threads or tasks and "wake" them up. For instance, if I want to close the server, I could notify the server to close, and the server will begin closing.
  
  Looking through this library also made me explore more about async rust which I probably wouldn't have done on the contrary, so a big thanks!