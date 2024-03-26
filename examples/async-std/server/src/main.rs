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
    let mut f = std::sync::Arc::new(conn);
    loop {
        // keeping the connection alive
        if f.is_closed().await {
            println!("Connection closed!");
            break;
        }

        if let Ok(pk) = std::sync::Arc::<Connection>::get_mut(&mut f).unwrap().recv().await {
            println!("(RAKNET RECIEVE SIDE) Got a connection packet {:?} ", pk);
            f.send(&[254, 12, 143, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0], true).await.unwrap();
        }
    }
}