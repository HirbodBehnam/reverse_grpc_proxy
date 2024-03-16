use std::net::SocketAddr;

use log::{debug, error};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, sync::mpsc, task};
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;

/// How many packets can be queued to be written in the socket
const SOCKET_CHAN_LENGTH: usize = 32;
/// How big is our read buffer size
const READ_BUFFER_SIZE: usize = 32 * 1024;

pub(crate) async fn create_new_connection(
    connection_id: uuid::Uuid,
    forward_address: SocketAddr,
    client: &mut super::proxy::proxy_controller_client::ProxyControllerClient<
        tonic::transport::Channel,
    >,
) {
    // Create a TCP socket to the forward address
    let forward_stream = TcpStream::connect(forward_address).await;
    if let Err(err) = forward_stream {
        error!("Cannot connect to forward address: {:?}", err);
        return;
    }
    let (mut forward_stream_read, mut forward_stream_write) = forward_stream.unwrap().into_split();
    // Do a gRPC request to the local
    let (data_sender, data_receiver) = mpsc::channel(SOCKET_CHAN_LENGTH);
    let request = Request::new(ReceiverStream::new(data_receiver));
    let proxy_response = client.proxy(request).await;
    if let Err(err) = proxy_response {
        error!("Cannot connect to local for {}: {}", connection_id, err);
        return;
    }
    let mut proxy_response = proxy_response.unwrap().into_inner();
    // Create two threads.
    // One is used to proxy from socket to gRPC
    task::spawn(async move {
        let mut read_buffer = [0u8; READ_BUFFER_SIZE];
        while let Ok(n) = forward_stream_read.read(&mut read_buffer).await {
            if let Err(err) = data_sender
                .send(crate::remote::proxy::TcpStreamPacket {
                    data: read_buffer[..n].to_owned(),
                })
                .await
            {
                // close the connection if socket is closed
                debug!(
                    "socket-{}: gRPC closed the connection: {}",
                    connection_id, err
                );
                break;
            }
        }
        debug!("socket-{}: Reader died", connection_id);
    });
    // Other is used to proxy from gRPC to socket
    task::spawn(async move {
        while let Ok(Some(data)) = proxy_response.message().await {
            if let Err(err) = forward_stream_write.write(data.data.as_slice()).await {
                // close the connection if socket is closed
                debug!(
                    "grpc-{}: Socket closed the connection: {}",
                    connection_id, err
                );
                break;
            }
        }
        debug!("grpc-{}: Reader died", connection_id);
    });
}
