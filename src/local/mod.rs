use std::{borrow::Borrow, sync::Arc};

use log::info;

mod grpc;
mod socket;

pub async fn start_local_server(cf_listen_address: &str, local_listen_address: &str) {
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
        tonic::transport::Server::builder()
            .add_service(reverse_proxy_local_service)
            .serve(cf_listen_socket)
            .await
            .expect("cannot listen for cloudflare");
    });

    // In main thread, wait for TCP sockets
    info!("Local listen is {local_listen_address}");
    socket::handle_socket(&local_listen_address, reverse_proxy_local.borrow()).await;
}
