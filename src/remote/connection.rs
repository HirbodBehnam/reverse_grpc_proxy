pub(crate) async fn create_new_connection(
    connection_id: uuid::Uuid,
    client: &mut super::proxy::proxy_controller_client::ProxyControllerClient<
        tonic::transport::Channel,
    >,
) {
    // Create a TCP socket to the forward address

    // Do a request to the local
}
