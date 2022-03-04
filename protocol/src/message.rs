use super::schema::*;
use super::MAX_MESSAGE_SIZE;

use prost::Message as _;
use std::fmt;
use std::io;
use hex;

/// Error if the buffer has insufficient size to encode a message.
#[derive(Debug)]
pub struct EncodeError {
    required: usize,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cannot encode message: Write buffer is full. ({})",
            self.required,
            )
    }
}

impl EncodeError {
    fn new(required: usize) -> Self {
        Self { required }
    }
}

impl From<prost::EncodeError> for EncodeError {
    fn from(e: prost::EncodeError) -> Self {
        Self::new(e.required_capacity())
    }
}

impl From<EncodeError> for io::Error {
    fn from(e: EncodeError) -> Self {
        io::Error::new(io::ErrorKind::Other, format!("{}", e))
    }
}

/// Encode data into a buffer.
///
/// This trait is implemented on data frames and their components
/// (channel messages, messages, and individual message types through prost).
pub trait Encoder: Sized + fmt::Debug {
    /// Calculates the length that the encoded message needs.
    fn encoded_len(&self) -> usize;

    /// Encodes the message to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError>;
}

impl Encoder for &[u8] {
    fn encoded_len(&self) -> usize {
        self.len()
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let len = self.encoded_len();
        if len > buf.len() {
            return Err(EncodeError::new(len));
        }
        buf[..len].copy_from_slice(&self[..]);
        Ok(len)
    }
}

/// The type of a data frame.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameType {
    Raw,
    Message,
}

/// A frame of data, either a buffer or a message.
#[derive(Clone, PartialEq)]
pub enum Frame {
    /// A raw binary buffer. Used in the handshaking phase.
    Raw(Vec<u8>),
    /// A message. Used for everything after the handshake.
    Message(ChannelMessage),
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Frame::Raw(buf) => write!(f, "Frame(Raw <{}>)", buf.len()),
            Frame::Message(message) => write!(f, "Frame({:?})", message),
        }
    }
}

impl From<ChannelMessage> for Frame {
    fn from(m: ChannelMessage) -> Self {
        Self::Message(m)
    }
}

impl From<Vec<u8>> for Frame {
    fn from(m: Vec<u8>) -> Self {
        Self::Raw(m)
    }
}

impl Frame {
    /// Decode a frame from a buffer.
    pub fn decode(buf: &[u8], frame_type: &FrameType) -> Result<Self, io::Error> {
        match frame_type {
            FrameType::Raw => Ok(Frame::Raw(buf.to_vec())),
            FrameType::Message => Ok(Frame::Message(ChannelMessage::decode(buf)?)),
        }
    }

    fn body_len(&self) -> usize {
        match self {
            Self::Raw(message) => message.as_slice().encoded_len(),
            Self::Message(message) => message.encoded_len(),
        }
    }
}

impl Encoder for Frame {
    fn encoded_len(&self) -> usize {
        let body_len = self.body_len();
        body_len + varinteger::length(body_len as u64)
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let len = self.encoded_len();
        if buf.len() < len {
            return Err(EncodeError::new(len));
        }
        let body_len = self.body_len();
        let header_len = len - body_len;
        varinteger::encode(body_len as u64, &mut buf[..header_len]);
        match self {
            Self::Raw(ref message) => message.as_slice().encode(&mut buf[header_len..]),
            Self::Message(ref message) => message.encode(&mut buf[header_len..]),
        }?;
        Ok(len)
    }
}

/// A protocol message.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Open a channel.
    Open(Open),
    /// Close a channel.
    Close(Close),
    /// Request a block.
    Request(Request),
    /// Send a Data block.
    Data(Data),
}

impl Message {
    /// Decode a message from a buffer.
    pub fn decode(buf: &[u8], typ: u64) -> io::Result<Self> {
        match typ {
            0 => Ok(Self::Open(Open::decode(buf)?)),
            1 => Ok(Self::Close(Close::decode(buf)?)),
            2 => Ok(Self::Request(Request::decode(buf)?)),
            3 => Ok(Self::Data(Data::decode(buf)?)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid message type",
            )),
        }
    }
    /// Wire type of this message.
    pub fn typ(&self) -> u64 {
        match self {
            Self::Open(_) => 0,
            Self::Close(_) => 1,
            Self::Request(_) => 2,
            Self::Data(_) => 3,
        }
    }
}

impl Encoder for Message {
    fn encoded_len(&self) -> usize {
        match self {
            Self::Open(ref message) => message.encoded_len(),
            Self::Close(ref message) => message.encoded_len(),
            Self::Request(ref message) => message.encoded_len(),
            Self::Data(ref message) => message.encoded_len(),
        }
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        match self {
            Self::Open(ref message) => encode_prost_message(message, buf),
            Self::Close(ref message) => encode_prost_message(message, buf),
            Self::Request(ref message) => encode_prost_message(message, buf),
            Self::Data(ref message) => encode_prost_message(message, buf),
        }
    }
}

fn encode_prost_message(
    msg: &impl prost::Message,
    mut buf: &mut [u8],
) -> Result<usize, EncodeError> {
    let len = msg.encoded_len();
    msg.encode(&mut buf)?;
    Ok(len)
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(msg) => write!(
                f,
                "Open(discovery_key: {}, capability: <{}>)",
                hex::encode(&msg.discovery_key),
                msg.capability.as_ref().map_or(0, |c| c.len()),
            ),
            Self::Close(msg) => write!(
                f,
                "Close(discovery_key: {})",
                hex::encode(&msg.discovery_key),
            ),
            Self::Request(msg) => write!(
                f,
                "Request(index: {})",
                msg.index,
            ),
            Self::Data(msg) => write!(
                f,
                "Data(index: {}, data: <{}>, data_signature: <{}>, tree_signature: <{}>)",
                msg.index,
                msg.data.len(),
                msg.data_signature.len(),
                msg.tree_signature.len(),
            ),
        }
    }
}

/// A message on a channel.
#[derive(Clone, PartialEq)]
pub struct ChannelMessage {
    pub channel: u64,
    pub message: Message,
}

impl fmt::Debug for ChannelMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChannelMessage({}, {})", self.channel, self.message)
    }
}

impl ChannelMessage {
    /// Create a new message.
    pub fn new(channel: u64, message: Message) -> Self {
        Self { channel, message }
    }

    /// Consume self and return (channel, Message).
    pub fn into_split(self) -> (u64, Message) {
        (self.channel, self.message)
    }

    /// Decode a channel message from a buffer.
    ///
    /// Note: `buf` has to have a valid length, and the length
    /// prefix has to be removed already.
    pub fn decode(buf: &[u8]) -> io::Result<Self> {
        if buf.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "received empty message",
            ));
        }
        let mut header = 0u64;
        let headerlen = varinteger::decode(&buf, &mut header);
        // let body = buf.split_off(headerlen);
        let channel = header >> 4;
        let typ = header & 0b1111;
        let message = Message::decode(&buf[headerlen..], typ)?;

        let channel_message = Self { channel, message };

        Ok(channel_message)
    }

    fn header(&self) -> u64 {
        let typ = self.message.typ();
        self.channel << 4 | typ
    }
}

impl Encoder for ChannelMessage {
    fn encoded_len(&self) -> usize {
        let header_len = varinteger::length(self.header());
        let body_len = self.message.encoded_len();
        header_len + body_len
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let header = self.header();
        let header_len = varinteger::length(header);
        let body_len = self.message.encoded_len();
        let len = header_len + body_len;
        if buf.len() < len || len > MAX_MESSAGE_SIZE as usize {
            return Err(EncodeError::new(len));
        }
        varinteger::encode(header, &mut buf[..header_len]);
        self.message.encode(&mut buf[header_len..len])?;
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! message_enc_dec {
        ($( $msg:expr ),*) => {
            $(
                let channel = rand::random::<u8>() as u64;
                let channel_message = ChannelMessage::new(channel, $msg);
                let mut buf = vec![0u8; channel_message.encoded_len()];
                let n = channel_message.encode(&mut buf[..])
                    .expect("Failed to encode message");
                let decoded = ChannelMessage::decode(&buf[..n])
                    .expect("Failed to decode message")
                    .into_split();
                assert_eq!(channel, decoded.0);
                assert_eq!($msg, decoded.1);
            )*
        }
    }

    #[test]
    fn encode_decode() {
        message_enc_dec! {
            Message::Open(Open {
                discovery_key: vec![2u8; 20],
                capability: None
            }),
            Message::Close(Close {
                discovery_key: vec![1u8; 10]
            }),
            Message::Request(Request {
                index: 0,
            }),
            Message::Data(Data {
                index: 1,
                data: vec![0u8; 10],
                data_signature: vec![1u8; 32],
                tree_signature: vec![2u8; 32],
            })
        };
    }
}
