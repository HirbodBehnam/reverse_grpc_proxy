# Reverse gRPC proxy
A TCP over gRPC port forwarder that the server sends the SYN packet.

## What?
A TCP port forwarder which wraps each TCP connection in a gRPC connection and the destination server establishes the gRPC connection. Mostly, this program works just like [reverse_ws_proxy](https://github.com/HirbodBehnam/reverse_ws_proxy) but everything is wrapped in gRPC connection with optional TLS.

### How?
At first let us assume that there are two applications:
1. Local: It wants to establish connections to a service of Remote server.
2. Remote: Serves an service which the Local client what's to connect to.
However, for any reason, you want the Remote server to send the SYN packet to Local client. So, the Remote server established a websocket connection with Local client called "Control". In this stream, the local sends an UUID for each incoming TCP connection. Then the server opens a websocket connection for each TCP connection and sends the UUID as the first packet. The client then forwards each packet of the TCP connection into the corresponding websocket stream.

## Running
### Building
It should be possible to just build this application with `cargo build --release`. No more configuration is needed.

### No TLS (h2c) client and server

Running the client and server without TLS is very simple. You just need to not use any of the tls related arguments. Here is an example:

#### Local Client
```bash
./reverse_grpc_proxy local --tcp-listen-address 127.0.0.1:12345 --cloudflare-listen-address 127.0.0.1:23456
```
#### Remote Server
Remember to mark use the `http://` prefix to indicate that we are not using TLS.

```bash
./reverse_grpc_proxy remote --cloudflare-server-address http://127.0.0.1:23456 --forward-address 127.0.0.1:54321
```

### TLS

If you are wishing to use TLS, you need the certificate and key in the local client and if you have used a self signed certificate, the certificate authority of it in the remote server.

#### Creating Certificates with OpenSSL

Note that the DNS.1 must be your server name.

```bash
openssl req -x509 -newkey rsa:2048 -keyout ca-key.pem -out ca-cert.pem -nodes -days 3650
cat <<'EOF' >> localhost.ext
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
subjectAltName = @alt_names
[alt_names]
DNS.1 = my.server.com
EOF
openssl req -nodes -newkey rsa:2048 -keyout key.pem -out key.csr
openssl x509 -req -CA ca-cert.pem -CAkey ca-key.pem -in key.csr -out cert.pem -days 3650 -CAcreateserial -extfile localhost.ext
```

#### Local Client
```bash
./reverse_grpc_proxy local --tcp-listen-address 127.0.0.1:12345 --cloudflare-listen-address 127.0.0.1:23456 --cert cert.pem --key key.pem 
```
#### Remote Server
Remember to mark use the `https://` prefix to indicate that we are using TLS.

```bash
./reverse_grpc_proxy remote --cloudflare-server-address https://127.0.0.1:23456 --forward-address 127.0.0.1:54321 --tls_ca ca-cert.pem --sni my.server.com
```