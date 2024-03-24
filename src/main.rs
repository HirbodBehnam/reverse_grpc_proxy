use clap::Parser;

mod arguments;
mod local;
mod remote;
mod util;

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = arguments::Args::parse();
    // Start the server or client
    match args.command {
        arguments::Commands::Local {
            tcp_listen_address,
            cloudflare_listen_address,
            tls_cert,
            tls_key,
        } => local::start_local_server(&cloudflare_listen_address, &tcp_listen_address, tls_cert, tls_key).await,
        arguments::Commands::Remote {
            cloudflare_server_address,
            forward_address,
            certificate_authority,
            domain,
        } => remote::start_remote_server(cloudflare_server_address, &forward_address, certificate_authority.as_ref(), domain).await,
    };
}
