use std::{net::SocketAddr, path::PathBuf, time::Duration};

use log::{debug, info, trace, warn};
use proxy::proxy_controller_client::ProxyControllerClient;
use tokio_stream::StreamExt;
use tonic::{
    transport::{Certificate, ClientTlsConfig},
    Request,
};

use crate::remote::connection::create_new_connection;

mod connection;

pub mod proxy {
    tonic::include_proto!("proxy");
}

pub async fn start_remote_server(
    controller_endpoint: String,
    forward_address: &str,
    ca_address: Option<&PathBuf>,
    domain: Option<String>,
) {
    // Leak values because why not
    let controller_endpoint: &'static str = Box::leak(Box::new(controller_endpoint));
    let forward_address: SocketAddr = forward_address
        .parse()
        .expect("cannot parse the forward address");
    // Create the TLS config if needed
    let tls_config = if ca_address.is_some() || domain.is_some() {
        let mut tls = ClientTlsConfig::new();
        if let Some(ca_address) = ca_address {
            let ca = tokio::fs::read_to_string(ca_address)
                .await
                .expect("cannot read the CA");
            let ca = Certificate::from_pem(ca);
            tls = tls.ca_certificate(ca);
            trace!("Loaded ca certificate");
        }
        if let Some(domain) = domain {
            trace!("Using {} as domain", domain);
            tls = tls.domain_name(domain);
        }
        debug!("Using custom TLS config");
        Some(tls)
    } else {
        debug!("Using default TLS config");
        None
    };
    // Create the endpoint
    let mut endpoint = tonic::transport::Endpoint::from_static(controller_endpoint);
    if let Some(tls_config) = tls_config {
        endpoint = endpoint
            .tls_config(tls_config)
            .expect("cannot set the TLS config");
    }
    // Loop because we want to retry connecting to controller
    loop {
        // Dummy wait to disable the burst retries
        tokio::time::sleep(Duration::from_secs(5)).await;
        // Create a gRPC client
        info!("Trying to connect to controller at {}", controller_endpoint);
        let endpoint = endpoint.connect().await;
        if let Err(err) = endpoint {
            warn!("Cannot connect to controller: {:?}", err);
            continue;
        }
        let mut client = ProxyControllerClient::new(endpoint.unwrap());
        info!("Connected to controller");
        // Connect to endpoint of controller
        let requests = client
            .controller(Request::new(proxy::ControllerRequest::default()))
            .await;
        if let Err(err) = requests {
            warn!("Cannot connect to controller endpoint: {:?}", err);
            continue;
        }
        // Read each request
        let mut requests = requests.unwrap().into_inner();
        while let Some(request) = requests.next().await {
            if let Err(err) = request {
                warn!("Controller disconnected: {:?}", err);
                break;
            }
            // Parse the connection ID
            let connection_id = request.unwrap().requested_connection_id.unwrap();
            let connection_id = uuid::Uuid::from_slice(connection_id.id.as_slice());
            if let Err(err) = connection_id {
                warn!("Invalid UUID: {:?}", err);
                continue;
            }
            let connection_id = connection_id.unwrap();
            // Create a new connection to the local
            debug!("New connection request: {}", connection_id);
            create_new_connection(connection_id, forward_address, &mut client).await;
        }
    }
}
