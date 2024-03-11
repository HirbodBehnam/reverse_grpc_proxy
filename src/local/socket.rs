use log::{debug, warn};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
};
use uuid::Uuid;

/// How many packets can be queued in the socket queue
const SOCKET_QUEUE_LENGTH: usize = 32;
/// How big is our read buffer size
const READ_BUFFER_SIZE: usize = 32 * 1024;

pub(crate) async fn handle_socket(listen: &str, reverse_proxy: &super::grpc::ReverseProxyLocal) {
    // Create the socket and listen
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .expect("cannot bind the TCP socket");
    loop {
        let (socket, socket_address) = listener.accept().await.expect("cannot accept connections");
        // For each socket, create a new UUID
        let socket_id = Uuid::new_v4();
        debug!("Accepted connection {socket_address} associated with {socket_id}");
        // Create the pipes and add the request in the pending sockets
        let (socket_sender, socket_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
        let (grpc_sender, grpc_receiver) = mpsc::channel(SOCKET_QUEUE_LENGTH);
        if reverse_proxy
            .register_socket(socket_id, grpc_sender, socket_receiver)
            .await
            == false
        {
            warn!("Control stream not established yet...");
            continue;
        }
        // Wait for acceptance
        tokio::task::spawn(handle_opened_socket(
            socket,
            socket_id,
            socket_sender,
            grpc_receiver,
        ));
    }
}

async fn handle_opened_socket(
    socket: TcpStream,
    socket_id: Uuid,
    socket_sender: Sender<Vec<u8>>,
    mut grpc_receiver: Receiver<Vec<u8>>,
) {
    // We dont need to wait for the websocket, just send the data in the pipes and hope for the best.
    let (mut socket_r, mut socket_w) = socket.into_split();
    // First spawn a task that only reads the data from the socket
    let mut socket_reader_task = tokio::task::spawn(async move {
        let mut read_buffer = [0u8; READ_BUFFER_SIZE];
        while let Ok(n) = socket_r.read(&mut read_buffer).await {
            socket_sender
                .send(read_buffer[..n].to_owned())
                .await
                .unwrap();
        }
        debug!("Socket {socket_id} closed on read");
    });
    // Now in a loop, wait for either a received packet from websocket or reader finishing
    loop {
        tokio::select! {
            // If the socket_reader_task is done, we can simply bail
            _ = (&mut socket_reader_task) => return,
            // But also check for commands
            data = grpc_receiver.recv() => {
                match data {
                    Some(data) => { // if there is data, write it into the pipe
                        socket_w.write(&data).await.unwrap();
                    }
                    None => { // websocket closed
                        socket_reader_task.abort();
                        debug!("Socket {socket_id} closed on write");
                        return; // socket_w will be dropped and connection will be closed
                    }
                }
            }
        }
    }
}
