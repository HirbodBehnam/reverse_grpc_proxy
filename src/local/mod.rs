use std::os::unix::net::SocketAddr;

use log::info;

mod grpc;
mod socket;

pub async fn start_local_server(cf_listen_address: &str, local_listen_address: &str) {
    // Create shared states.
    // We can simply leak these values to do not pay for reference counting because we need them for the rest of the program.
    let reverse_proxy_local: &'static grpc::ReverseProxyLocal =
        Box::leak(Box::new(grpc::ReverseProxyLocal::default()));

    // Tell tonic to run the app
    tokio::spawn(async move {
        let cf_listen_socket = cf_listen_address
            .parse()
            .expect("cannot parse the socket address");
        info!("Cloudflare listen is {cf_listen_address}");
        tonic::transport::Server::builder()
            .add_service(grpc::ReverseProxyLocal::default())
            .serve(cf_listen_socket)
            .await;
    });

    // In main thread, wait for TCP sockets
    info!("Local listen is {local_listen_address}");
    socket::handle_socket(&local_listen_address, reverse_proxy_local).await;
}
