use log::{debug, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
};
use uuid::Uuid;

use crate::util::{READ_BUFFER_SIZE, SOCKET_CHAN_LENGTH};

pub(crate) async fn handle_socket(listen: &str, reverse_proxy: &super::grpc::ReverseProxyLocal) {
    // Create the socket and listen
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .expect("cannot bind the TCP socket");
    loop {
        let (socket, socket_address) = listener.accept().await.expect("cannot accept connections");
        // For each socket, create a new UUID
        let socket_id = Uuid::new_v4();
        debug!(
            "socket-{}: accepted connection from {}",
            socket_id, socket_address
        );
        // Create the pipes and add the request in the pending sockets
        let (socket_sender, socket_receiver) = mpsc::channel(SOCKET_CHAN_LENGTH);
        let (grpc_sender, grpc_receiver) = mpsc::channel(SOCKET_CHAN_LENGTH);
        if reverse_proxy
            .register_socket(socket_id, grpc_sender, socket_receiver)
            .await
            == false
        {
            warn!("control stream not established yet...");
            continue;
        }
        // Wait for acceptance
        handle_opened_socket(socket, socket_id, socket_sender, grpc_receiver).await;
    }
}

async fn handle_opened_socket(
    socket: TcpStream,
    socket_id: Uuid,
    socket_sender: Sender<Vec<u8>>,
    mut grpc_receiver: Receiver<Vec<u8>>,
) {
    // We don't need to wait for the websocket, just send the data in the pipes and hope for the best.
    let (mut socket_r, mut socket_w) = socket.into_split();
    // We use one task to read from socket and send data to gRPC
    tokio::task::spawn(async move {
        let mut read_buffer = [0u8; READ_BUFFER_SIZE];
        while let Ok(n) = socket_r.read(&mut read_buffer).await {
            if n == 0 {
                break;
            }
            if socket_sender
                .send(read_buffer[..n].to_owned())
                .await
                .is_err()
            {
                break;
            }
        }
        debug!("socket-{}: closed on read", socket_id);
    });
    // And another task to read data from gRPC and send into socket
    tokio::task::spawn(async move {
        while let Some(data) = grpc_receiver.recv().await {
            if let Err(err) = socket_w.write(&data).await {
                debug!("socket-{}: cannot write data in socket {}", socket_id, err);
                break;
            }
        }
        let _ = socket_w.shutdown().await;
        debug!("socket-{}: closed on write", socket_id);
    });
}
