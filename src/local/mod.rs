use std::{borrow::Borrow, path::PathBuf, sync::Arc};

use log::{debug, info};

mod grpc;
mod socket;

pub async fn start_local_server(
    cf_listen_address: &str,
    local_listen_address: &str,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
) {
    // Create shared states.
    // We can simply leak these values to do not pay for reference counting because we need them for the rest of the program.
    let reverse_proxy_local = Arc::new(grpc::ReverseProxyLocal::default());

    // Tell tonic to run the app
    let cf_listen_socket = cf_listen_address
        .parse()
        .expect("cannot parse the socket address");
    info!("Cloudflare listen is {cf_listen_address}");
    let reverse_proxy_local_service =
        grpc::proxy::proxy_controller_server::ProxyControllerServer::from_arc(Arc::clone(
            &reverse_proxy_local,
        ));
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        // Add TLS
        if tls_cert.is_some() && tls_key.is_some() {
            let cert = tokio::fs::read_to_string(tls_cert.unwrap())
                .await
                .expect("cannot read the cert file");
            let key = tokio::fs::read_to_string(tls_key.unwrap())
                .await
                .expect("cannot read the key file");
            let identity = tonic::transport::Identity::from_pem(cert, key);
            builder = builder
                .tls_config(tonic::transport::ServerTlsConfig::new().identity(identity))
                .expect("cannot set the TLS config");
            debug!("Using TLS");
        } else {
            debug!("Not using TLS");
        }
        // Add the gRPC service
        builder
            .add_service(reverse_proxy_local_service)
            .serve(cf_listen_socket)
            .await
            .expect("cannot listen for cloudflare");
    });

    // In main thread, wait for TCP sockets
    info!("Local listen is {local_listen_address}");
    socket::handle_socket(&local_listen_address, reverse_proxy_local.borrow()).await;
}
