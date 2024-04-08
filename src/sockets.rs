

use futures::SinkExt;
use futures::StreamExt;
use serde::Serialize;
use tokio::net::{TcpListener, TcpStream};

use crate::family_tree::FamilyTree;
use crate::BotScore;
use crate::{Bot, Species};

#[derive(Clone, Serialize, Debug)]
pub struct TrainingProgressAnnouncement {
    pub best_bot: Bot,
    pub species: FamilyTree,
    pub iteration_number: usize,
    pub last_game: Vec<String>
}

pub async fn start_socket(
    receiver: tokio::sync::broadcast::Sender<TrainingProgressAnnouncement>,
) -> Result<(), std::io::Error> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream, receiver.subscribe()));
    }

    Ok(())
}

async fn accept_connection(
    stream: TcpStream,
    mut listener: tokio::sync::broadcast::Receiver<TrainingProgressAnnouncement>,
) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", addr);

    let (mut write, mut read) = ws_stream.split();

    tokio::spawn(async move {
        while let Some(_) = read.next().await {
            // ignore incoming messages
        }
    });

    while let Ok(message) = listener.recv().await {
        if let Err(e) = write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::to_string(&message).unwrap(),
            ))
            .await
        {
            eprint!("Connection closed, {e:?}");
            break;
        }
    }
    // We should not forward messages other than text or binary.
}
