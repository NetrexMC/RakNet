# RakNet
A fully functional RakNet implementation in rust.



#### Implementing packets using `binary_utils`

```rust
use binary_utils::*;
use rakrs::Magic;

#[derive(BinaryStream)]
pub struct SomeData {
    pub name: String,
    pub is_banned: bool
}

#[derive(BinaryStream)]
pub struct MyPacket {
    pub id: u8,
    pub magic: Magic,
    pub data: SomeData
}
```

#### Starting a RakNet Server

```rust
use rakrs::Server as RakServer;

fn main() {
    let mut server = RakServer::new("0.0.0.0:19132".into());
    
    // Setting the Message Of The Day
    server.set_motd(Motd {
        name: "Server Name".into(),
        protocol: 420,
        player_count: 0,
        player_max: 10,
        gamemode: "Creative".into(),
        version: "1.18.0".into(),
        server_id: server.server_id.into()
    });
    
    let join_fn = Arc::new(|_con: &mut Connection, packet: &mut Vec<u8>| {
        println!("Gamepacket was recieved!");
    });
    
    let event_fn = Box::new(|event: &RakNetEvent| {
        match *event {
            RakNetEvent::Disconnect(address, reason) => {
                println!("{} was disconnected due to: {}", address, reason);
            },
            RakNetEvent::ConnectionCreated(address) => {
                println!("{} has joined the server.");
            },
            _ => return
        }
    });
    
    raknet_start!(server, join_fn, event_fn);
}
```

