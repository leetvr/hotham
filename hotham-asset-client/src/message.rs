use anyhow::Result;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    GetAsset,
    WatchAsset,
    AssetUpdated,
    OK,
    Error,
    Asset,
    _Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<'a> {
    GetAsset(&'a str),
    WatchAsset(&'a str),
    AssetUpdated(&'a str),
    OK,
    Error(String),
    Asset(Vec<u8>),
}

impl<'a> Message<'a> {
    pub fn parse(message_type: MessageType, buffer: &'a [u8]) -> Result<Message<'a>> {
        let message = match message_type {
            MessageType::GetAsset => Message::GetAsset(std::str::from_utf8(buffer)?),
            MessageType::WatchAsset => Message::WatchAsset(std::str::from_utf8(buffer)?),
            MessageType::AssetUpdated => Message::AssetUpdated(std::str::from_utf8(buffer)?),
            MessageType::OK => Message::OK,
            MessageType::Error => Message::Error(std::str::from_utf8(buffer)?.into()),
            MessageType::Asset => Message::Asset(buffer.to_vec()),
            _ => anyhow::bail!("Invalid message type"),
        };

        Ok(message)
    }

    pub async fn read(recv: &mut quinn::RecvStream, buffer: &'a mut [u8]) -> Result<Message<'a>> {
        // Read 9 bytes
        recv.read_exact(&mut buffer[0..9]).await?;
        // Byte 0 is the tag:
        let message_type: MessageType = unsafe { std::mem::transmute(buffer[0]) };
        // Bytes 1..9 is the message length as big endian u64
        let message_length = u64::from_be_bytes(buffer[1..9].try_into()?);

        let message_buffer = &mut buffer[0..message_length as _];
        recv.read_exact(message_buffer).await?;
        Message::parse(message_type, message_buffer)
    }

    pub async fn write_all(&'a self, stream: &mut quinn::SendStream) -> Result<()> {
        // Would it be easier to just put this all into another buffer?
        // Yes
        // Would it be as fun?
        // No.

        // Byte 0 is the tag:
        stream.write_all(&[self.get_type() as _]).await?;
        // Bytes 1..9 is the message length as big endian u64
        stream.write_all(&(self.len() as u64).to_be_bytes()).await?;
        stream.write_all(self.buf()).await?;

        Ok(())
    }

    pub fn get_type(&self) -> MessageType {
        match self {
            Message::GetAsset(_) => MessageType::GetAsset,
            Message::WatchAsset(_) => MessageType::WatchAsset,
            Message::AssetUpdated(_) => MessageType::AssetUpdated,
            Message::OK => MessageType::OK,
            Message::Error(_) => MessageType::Error,
            Message::Asset(_) => MessageType::Asset,
        }
    }

    pub fn len(&self) -> usize {
        self.buf().len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf().is_empty()
    }

    pub fn buf(&self) -> &'_ [u8] {
        match self {
            Message::GetAsset(s) => s.as_bytes(),
            Message::WatchAsset(s) => s.as_bytes(),
            Message::AssetUpdated(s) => s.as_bytes(),
            Message::OK => &[],
            Message::Error(s) => s.as_bytes(),
            Message::Asset(b) => b,
        }
    }
}
