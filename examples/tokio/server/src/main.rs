use rak_rs::Listener;
use rak_rs::Motd;
use rak_rs::connection::Connection;
use rak_rs::mcpe;
use rak_rs::mcpe::motd::Gamemode;
use rak_rs::server::event::ServerEvent;
use rak_rs::server::event::ServerEventResponse;


#[tokio::main]
async fn main() {
    console_subscriber::init();
    let mut server = Listener::bind("0.0.0.0:19132").await.unwrap();
    server.motd.name = "RakNet Rust (tokio)!".to_string();
    server.motd.gamemode = Gamemode::Survival;

    server.start().await.unwrap();

    loop {
        let conn = server.accept().await;
        tokio::task::spawn(handle(conn.unwrap()));
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
        }
    }
}