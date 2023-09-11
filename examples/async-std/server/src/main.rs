use rak_rs::connection::Connection;
use rak_rs::mcpe::motd::Gamemode;
use rak_rs::Listener;

#[async_std::main]
async fn main() {
    // console_subscriber::init();
    let mut server = Listener::bind("0.0.0.0:19133").await.unwrap();
    server.motd.name = "RakNet Rust (async-std)!".to_string();
    server.motd.gamemode = Gamemode::Survival;
    server.motd.player_count = 69420;
    server.motd.player_max = 69424;

    server.start().await.unwrap();

    loop {
        let conn = server.accept().await;
        async_std::task::spawn(handle(conn.unwrap()));
    }
}

async fn handle(mut conn: Connection) {
    loop {
        // keeping the connection alive
        if conn.is_closed().await {
            println!("Connection closed!");
            break;
        }
        if let Ok(pk) = conn.recv().await {
            println!("(RAKNET RECIEVE SIDE) Got a connection packet {:?} ", pk);
            conn.send(&[254, 12, 143, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0], true).await.unwrap();
        }
    }
}


// GOOD RAKNET: 8004000060007000000000000000fe0c8f0101000000000000000000
// BAD RAKNET:  8003000060006804000001000000fe0c8f01010000000000000000

// good         8004000060007000000000000000fe0c8f0101000000000000000000


// GOOD
// 128, 4, 0, 0, 96, 0, 112, 0, 0, 0, 0, 0, 0, 0, 254, 12, 143, 1, 1, 0, 0, 0, 0, 0, 0, 0

// 0000   80 04 00 00 60 00 70 00 00 00 00 00 00 00

// mine
// 8003000060006004000000000000fe0c8f010100000000000000
// pmmp
// 8004000060007000000000000000fe0c8f0101000000000000000000
// goraknet
// 8403000060006003000003000000fe0c8f010100000000000000


// 0200000045000023877e0000801100007f0000017f0000014abdf0d4000ffc38c00100010a0000

// 0200000045000023878b0000801100007f0000017f000001f0d44abd000ffe39c0000101070000


// 13
//02000000450000234e1c0000801100007f0000017f0000014abdf0d4000ff938c00100010d0000
//02000000450000234e6c0000801100007f0000017f000001f0d44abd000ff839c00001010d0000