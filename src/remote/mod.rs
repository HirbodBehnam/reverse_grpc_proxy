use std::{net::SocketAddr, time::Duration};

use log::{debug, info, warn};
use proxy::proxy_controller_client::ProxyControllerClient;
use tokio_stream::StreamExt;
use tonic::Request;

use crate::remote::connection::create_new_connection;

mod connection;

pub mod proxy {
    tonic::include_proto!("proxy");
}

pub async fn start_remote_server(controller_endpoint: String, forward_address: &str) {
    // Leak values because why not
    let controller_endpoint: &'static str = Box::leak(Box::new(controller_endpoint));
    let forward_address: SocketAddr = forward_address.parse().expect("cannot parse the forward address");
    // Loop because we want to retry connecting to controller
    loop {
        // Dummy wait to disable the burst retries
        tokio::time::sleep(Duration::from_secs(5)).await;
        // Create a gRPC client
        info!("Trying to connect to controller at {}", controller_endpoint);
        let client = ProxyControllerClient::connect(controller_endpoint).await;
        if let Err(err) = client {
            warn!("Cannot connect to controller: {}", err);
            continue;
        }
        info!("Connected to controller");
        let mut client = client.unwrap();
        // Connect to endpoint of controller
        let requests = client
            .controller(Request::new(proxy::ControllerRequest::default()))
            .await;
        if let Err(err) = requests {
            warn!("Cannot connect to controller endpoint: {}", err);
            continue;
        }
        // Read each request
        let mut requests = requests.unwrap().into_inner();
        while let Some(request) = requests.next().await {
            if let Err(err) = request {
                warn!("Controller disconnected: {}", err);
                break;
            }
            // Parse the connection ID
            let connection_id = request.unwrap().requested_connection_id.unwrap();
            let connection_id = uuid::Uuid::from_slice(connection_id.id.as_slice());
            if let Err(err) = connection_id {
                warn!("Invalid UUID: {}", err);
                continue;
            }
            let connection_id = connection_id.unwrap();
            // Create a new connection to the local
            debug!("New connection request: {}", connection_id);
            create_new_connection(connection_id, forward_address, &mut client).await;
        }
    }
}
