use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
#[command(about = "A gRPC based reverse proxy", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Run as the server that cloudflare connects to", long_about = None)]
    Local {
        #[arg(short = 'l', long, help = "On what address we should listen and accept TCP connections?")]
        tcp_listen_address: String,
        #[arg(short = 'c', long, help = "On what address we should listen and accept the connections from Cloudflare?")]
        cloudflare_listen_address: String,
        #[arg(long = "cert", help = "The certificate path of the TLS configuration", requires = "tls_key")]
        tls_cert: Option<PathBuf>,
        #[arg(long = "key", help = "The key path of the TLS configuration", requires = "tls_cert")]
        tls_key: Option<PathBuf>,
    },
    #[command(about = "Run as the program that connects to cloudflare", long_about = None)]
    Remote {
        #[arg(short = 'c', long, help = "What is the address of cloudflare that we should send the gRPC to?")]
        cloudflare_server_address: String,
        #[arg(short = 'f', long, help = "Where we should forward the gRPC traffic?")]
        forward_address: String,
        #[arg(long = "tls_ca", help = "The certificate authority file if server certificate is self signed")]
        certificate_authority: Option<PathBuf>,
        #[arg(long = "sni", help = "The domain to connect to in the TLS config")]
        domain: Option<String>,
    }
}