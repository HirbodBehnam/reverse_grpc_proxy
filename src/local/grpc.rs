use std::{collections::HashMap, ops::DerefMut};

use self::proxy::{ConnectionId, ControllerResponse, ProxyRequest, TcpStreamPacket};
use crate::local::grpc::proxy::proxy_controller_server::ProxyController;
use log::{info, trace, warn};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;
use uuid::Uuid;

pub mod proxy {
    tonic::include_proto!("proxy");
}

/// How many messages can be queued in the CONTROLLER_COMMANDER
const CONTROLLER_COMMANDER_CHAN_LENGTH: usize = 16;

/// The data type that should be sent into the pipe
type ControllerStreamData = Result<ControllerResponse, Status>;

/// ConnectionPipe is used to connect a socket to a gRPC stream.
struct ConnectionPipe {
    /// gRPC stream sends into this pipe in order to send data in the socket
    grpc_data: mpsc::Sender<Vec<u8>>,
    /// gRPC stream await this pipe to get the data from the opened socket
    socket_data: mpsc::Receiver<Vec<u8>>,
}

#[derive(Default)]
pub(crate) struct ReverseProxyLocal {
    /// If a commander is connected, this option will be some with an end of a channel sender.
    /// Otherwise, it is null and indicates that the commander is not connected yet.
    controller_commander: tokio::sync::Mutex<Option<mpsc::Sender<ControllerStreamData>>>,
    /// A list of all pending connections that the remote should connect.
    pending_socket_connections: parking_lot::Mutex<HashMap<Uuid, ConnectionPipe>>,
}

impl ReverseProxyLocal {
    /// Sends a connection request in the commander.
    /// Returns true if the sent request was successful. Otherwise, returns false.
    /// False indicates that the commander is not connected.
    async fn request_connection(&self, connection_id: uuid::Uuid) -> bool {
        // Lock the thing
        let mut commander = self.controller_commander.lock().await;
        if commander.is_none() {
            return false;
        }
        // Send the data
        if commander
            .as_ref()
            .unwrap()
            .send(Ok(ControllerResponse {
                requested_connection_id: Some(ConnectionId {
                    id: connection_id.as_bytes().to_vec(),
                }),
            }))
            .await
            .is_err()
        {
            // Remove the commander
            *commander.deref_mut() = None;
            return false;
        }
        // We have sent the data!
        return true;
    }

    /// Request a socket from the controller. Returns false if the controller is not connected.
    pub(crate) async fn register_socket(
        &self,
        connection_id: uuid::Uuid,
        grpc_data: mpsc::Sender<Vec<u8>>,
        socket_data: mpsc::Receiver<Vec<u8>>,
    ) -> bool {
        // Save the pipes in the hashmap
        self.pending_socket_connections.lock().insert(
            connection_id,
            ConnectionPipe {
                grpc_data,
                socket_data,
            },
        );
        // Try to send a request to the controller
        if self.request_connection(connection_id).await == false {
            // Oops. The controller is dead
            self.pending_socket_connections.lock().remove(&connection_id);
            return false;
        }
        // Done!
        return true;
    }
}

#[tonic::async_trait]
impl ProxyController for ReverseProxyLocal {
    type ControllerStream = ReceiverStream<ControllerStreamData>;
    /// Controller endpoint will be triggered when the remote is trying to register
    /// as a controller.
    async fn controller(
        &self,
        _: tonic::Request<proxy::ControllerRequest>,
    ) -> Result<tonic::Response<Self::ControllerStream>, tonic::Status> {
        // We only allow on instance of the controller.
        let mut commander = self.controller_commander.lock().await;
        if commander.is_some() {
            drop(commander);
            warn!("Duplicate controller");
            // Well, no. LOL
            return Err(tonic::Status::already_exists("controller"));
        }
        // Create the channel
        let (command_sender, command_receiver) = mpsc::channel(CONTROLLER_COMMANDER_CHAN_LENGTH);
        *commander.deref_mut() = Some(command_sender);
        drop(commander);
        // Finalize the upgrade process by returning upgrade callback.
        info!("Detected a new commander");
        Ok(tonic::Response::new(ReceiverStream::new(command_receiver)))
    }

    type ProxyStream = ReceiverStream<Result<TcpStreamPacket, Status>>;
    async fn proxy(
        &self,
        request: tonic::Request<tonic::Streaming<ProxyRequest>>,
    ) -> Result<tonic::Response<Self::ProxyStream>, tonic::Status> {
        unimplemented!();
    }
}
