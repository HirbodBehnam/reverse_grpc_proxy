/// How many packets can be queued to be written in the socket
pub const SOCKET_CHAN_LENGTH: usize = 32;
/// How big is our read buffer size
pub const READ_BUFFER_SIZE: usize = 32 * 1024;
/// The metadata name in the header of the proxy request which indicates the
/// connection ID that the stream corresponds to.
pub const CONNECTION_ID_METADATA_NAME: &str = "x-connection-id";

/// Tries to read from a socket but with the ability to cancel it.
/// The socket must be a AsyncReadExt trait.
/// Buffer is simply a byte slice.
/// Cancel must be an awaitable that if it is awaited, it means that the
/// read operation must be canceled. In this project, I use a single shot receiver
macro_rules! read_with_cancel {
    ($socket:expr, $buffer:expr, $cancel:expr) => {
        tokio::select! {
            _ = &mut $cancel => {
                Err(std::io::ErrorKind::ConnectionReset.into())
            },
            res = $socket.read(&mut $buffer) => {
                res
            },
        }
    };
}

pub(crate) use read_with_cancel;