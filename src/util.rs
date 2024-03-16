/// How many packets can be queued to be written in the socket
pub const SOCKET_CHAN_LENGTH: usize = 32;
/// How big is our read buffer size
pub const READ_BUFFER_SIZE: usize = 32 * 1024;
/// The metadata name in the header of the proxy request which indicates the
/// connection ID that the stream corresponds to.
pub const CONNECTION_ID_METADATA_NAME: &str = "x-connection-id";