use rak_rs::{Listener, Motd};
use rak_rs::client::{Client, DEFAULT_MTU};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut listener = Listener::bind("0.0.0.0:19132").await.unwrap();

    // get address from process args
    let addr = match std::env::args().nth(1) {
        Some(a) => {
            a
        },
        None => {
            println!("Usage: {} <address>", std::env::args().nth(0).unwrap());
            return Ok(());
        }
    };

    println!("Starting proxy for {}", addr);

    // connect to server
    match Client::ping(addr.clone()).await {
        Err(e) => {
            println!("Could not identify remote: {}", e);
            return Ok(());
        }
        Ok(pong) => {
            println!("Found server server: {:?}", pong.motd.write());
            listener.motd = Motd::new(listener.id, "19132");
            listener.motd.player_count = pong.motd.player_count;
            listener.motd.player_max = pong.motd.player_max;
            listener.motd.name = pong.motd.name;
            listener.motd.gamemode = pong.motd.gamemode;
        }
    }

    listener.start().await?;

    loop {
        let mut conn = listener.accept().await?;
        tokio::spawn(async move {
            // first we try to connect to the server with a new client
            let addr = std::env::args().nth(1).unwrap();
            let mut client = Client::new(11, DEFAULT_MTU);
            if let Err(e) = client.connect(addr).await {
                println!("Could not connect to remote: {}", e);
                return;
            }

            // now we just forward patckets from the client to the server
            loop {
                tokio::select! {
                    Ok(packet) = conn.recv() => {
                        println!("S->C: {:?}", packet);
                        if let Err(e) = client.send_ord(&packet[..], 0).await {
                            println!("Could not send packet to remote: {}", e);
                            return;
                        }
                    }
                    Ok(packet) = client.recv() => {
                        println!("C->S: {:?}", packet);
                        if let Err(e) = conn.send(&packet[..], true).await {
                            println!("Could not send packet to client: {}", e);
                            return;
                        }
                    }
                }
            }

        });
    }
    Ok(())
}