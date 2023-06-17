use rakrs::Listener;
use rakrs::Motd;
use rakrs::connection::Connection;
use rakrs::mcpe;
use rakrs::mcpe::motd::Gamemode;
use rakrs::server::event::ServerEvent;
use rakrs::server::event::ServerEventResponse;


#[tokio::main]
async fn main() {
    console_subscriber::init();
    let mut server = Listener::bind("0.0.0.0:19132").await.unwrap();
    // let inner = server.recv_evnt.clone();
    server.motd.name = "RakNet Rust!".to_string();
    server.motd.gamemode = Gamemode::Survival;

    server.start().await.unwrap();

    loop {
        // let mut recvr = inner.lock().await;
        // tokio::select! {
        //     ev = recvr.recv() => {
        //         match ev {
        //             Some((event, shoot)) => {
        //                 match event {
        //                     ServerEvent::RefreshMotdRequest(_addr, mut motd) => {
        //                         // force update the motd!!!
        //                         motd.name = "You are a loser!!!!!!!!!".to_string();
        //                         motd.player_max = 14_903;
        //                         motd.player_count = 0;
        //                         shoot.send(ServerEventResponse::RefreshMotd(motd)).unwrap();
        //                     },
        //                     _ => {
        //                         println!("Got a response!");
        //                         // YOU NEED TO ALWAYS REPLY TO EVENTS
        //                         shoot.send(ServerEventResponse::Acknowledged);
        //                     }
        //                 }
        //             },
        //             None => {
        //                 println!("Error!");
        //                 break;
        //             }
        //         }
        //     },
            // connection = server.accept() => {
            //     tokio::task::spawn(handle(connection.unwrap()));
            // }

        let conn = server.accept().await;
        async_std::task::spawn(handle(conn.unwrap()));
        // }
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
            println!("(RAKNET RECIEVE SIDE) Got a connection packet {:?} ", pk);
        }
        // conn.tick().await;
    }
}