use std::ops::DerefMut;

use self::proxy::{ConnectionId, ControllerResponse, ProxyRequest, TcpStreamPacket};
use crate::local::proxy::proxy_controller_server::ProxyController;
use log::{info, trace, warn};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;

pub mod proxy {
    tonic::include_proto!("proxy");
}

/// How many messages can be queued in the CONTROLLER_COMMANDER
const CONTROLLER_COMMANDER_CHAN_LENGTH: usize = 16;

/// The data type that should be sent into the pipe
type ControllerStreamData = Result<ControllerResponse, Status>;

#[derive(Debug, Default)]
pub struct ReverseProxyLocal {
    controller_commander: Mutex<Option<mpsc::Sender<ControllerStreamData>>>,
}

impl ReverseProxyLocal {
    /// Sends a connection request in the commander.
    /// Returns true if the sent request was successful. Otherwise, returns false.
    /// False indicates that the commander is not connected.
    pub async fn request_connection(&self, connection_id: uuid::Uuid) -> bool {
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
}

#[tonic::async_trait]
impl ProxyController for ReverseProxyLocal {
    type ControllerStream = ReceiverStream<ControllerStreamData>;
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
