use crate::errors::JournalError;

/// Journal file magic bytes: `b"NRJ1"`.
pub const MAGIC: &[u8; 4] = b"NRJ1";

/// Current journal format version: `0x0001`.
pub const VERSION: u16 = 0x0001;

/// Header size in bytes: 16 bytes.
pub const HEADER_SIZE: usize = 16;

impl JournalHeader {
    /// Header size constant.
    pub const HEADER_SIZE: usize = 16;
}

/// Frame header size in bytes: 8 bytes.
pub const FRAME_HEADER_SIZE: usize = 8;

impl RecordFrame {
    /// Frame header size constant.
    pub const FRAME_HEADER_SIZE: usize = 8;
}

/// Maximum recommended payload size: 16 MiB.
pub const MAX_PAYLOAD_SIZE: u32 = 16 * 1024 * 1024;

/// Record frame kind: EventJson.
pub const FRAME_KIND_EVENT_JSON: u8 = 0x01;

/// Journal file header (16 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalHeader {
    /// Magic bytes: `"NRJ1"`.
    pub magic: [u8; 4],
    /// Format version: `0x0001`.
    pub version: u16,
    /// Reserved flags (must be 0).
    pub flags: u16,
    /// Reserved bytes (must be all zeros).
    pub reserved: [u8; 8],
}

impl JournalHeader {
    /// Creates a new header with default values.
    pub fn new() -> Self {
        Self {
            magic: *MAGIC,
            version: VERSION,
            flags: 0,
            reserved: [0; 8],
        }
    }

    /// Serializes the header to bytes.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4..6].copy_from_slice(&self.version.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.flags.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.reserved);
        bytes
    }

    /// Deserializes a header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JournalError> {
        if bytes.len() < HEADER_SIZE {
            return Err(JournalError::InvalidHeader(format!(
                "header too short: {} bytes",
                bytes.len()
            )));
        }

        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != *MAGIC {
            return Err(JournalError::InvalidHeader(format!(
                "invalid magic: {:?}, expected {:?}",
                magic, MAGIC
            )));
        }

        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != VERSION {
            return Err(JournalError::InvalidHeader(format!(
                "unsupported version: 0x{:04x}, expected 0x{:04x}",
                version, VERSION
            )));
        }

        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        if flags != 0 {
            return Err(JournalError::InvalidHeader(format!(
                "non-zero flags: 0x{:04x}",
                flags
            )));
        }

        let mut reserved = [0u8; 8];
        reserved.copy_from_slice(&bytes[8..16]);
        if reserved != [0u8; 8] {
            return Err(JournalError::InvalidHeader(
                "non-zero reserved bytes".to_string(),
            ));
        }

        Ok(Self {
            magic,
            version,
            flags,
            reserved,
        })
    }
}

impl Default for JournalHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// Record frame kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// EventJson: UTF-8 JSON object representing a canonical event.
    EventJson,
    /// Unknown/unsupported frame kind.
    Unknown(u8),
}

impl FrameKind {
    /// Creates a FrameKind from a byte value.
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            FRAME_KIND_EVENT_JSON => FrameKind::EventJson,
            _ => FrameKind::Unknown(byte),
        }
    }

    /// Returns the byte value for this kind.
    pub fn to_byte(self) -> u8 {
        match self {
            FrameKind::EventJson => FRAME_KIND_EVENT_JSON,
            FrameKind::Unknown(b) => b,
        }
    }
}

/// Record frame header (8 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFrame {
    /// Frame kind.
    pub kind: FrameKind,
    /// Reserved bytes (must be all zeros).
    pub reserved: [u8; 3],
    /// Payload length in bytes (little-endian).
    pub len: u32,
}

impl RecordFrame {
    /// Creates a new frame header.
    pub fn new(kind: FrameKind, len: u32) -> Result<Self, JournalError> {
        if len > MAX_PAYLOAD_SIZE {
            return Err(JournalError::PayloadTooLarge {
                size: len,
                max: MAX_PAYLOAD_SIZE,
            });
        }
        Ok(Self {
            kind,
            reserved: [0; 3],
            len,
        })
    }

    /// Serializes the frame header to bytes.
    pub fn to_bytes(&self) -> [u8; FRAME_HEADER_SIZE] {
        let mut bytes = [0u8; FRAME_HEADER_SIZE];
        bytes[0] = self.kind.to_byte();
        bytes[1..4].copy_from_slice(&self.reserved);
        bytes[4..8].copy_from_slice(&self.len.to_le_bytes());
        bytes
    }

    /// Deserializes a frame header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JournalError> {
        if bytes.len() < FRAME_HEADER_SIZE {
            return Err(JournalError::InvalidFrame {
                offset: 0,
                reason: format!("frame header too short: {} bytes", bytes.len()),
            });
        }

        let kind = FrameKind::from_byte(bytes[0]);
        let reserved = [bytes[1], bytes[2], bytes[3]];
        if reserved != [0u8; 3] {
            return Err(JournalError::InvalidFrame {
                offset: 0,
                reason: "non-zero reserved bytes".to_string(),
            });
        }
        let len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        if len > MAX_PAYLOAD_SIZE {
            return Err(JournalError::InvalidFrame {
                offset: 0,
                reason: format!("payload size {} exceeds maximum {}", len, MAX_PAYLOAD_SIZE),
            });
        }

        Ok(Self {
            kind,
            reserved,
            len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip() {
        let header = JournalHeader::new();
        let bytes = header.to_bytes();
        let restored = JournalHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, restored);
    }

    #[test]
    fn header_rejects_invalid_magic() {
        let mut bytes = JournalHeader::new().to_bytes();
        bytes[0] = b'X';
        assert!(JournalHeader::from_bytes(&bytes).is_err());
    }

    #[test]
    fn header_rejects_invalid_version() {
        let mut bytes = JournalHeader::new().to_bytes();
        bytes[4] = 0x02;
        bytes[5] = 0x00;
        assert!(JournalError::InvalidHeader(
            JournalHeader::from_bytes(&bytes).unwrap_err().to_string()
        )
        .to_string()
        .contains("version"));
    }

    #[test]
    fn header_rejects_non_zero_flags() {
        let mut bytes = JournalHeader::new().to_bytes();
        bytes[6] = 0x01;
        assert!(JournalHeader::from_bytes(&bytes).is_err());
    }

    #[test]
    fn header_rejects_non_zero_reserved() {
        let mut bytes = JournalHeader::new().to_bytes();
        bytes[8] = 0x01;
        assert!(JournalHeader::from_bytes(&bytes).is_err());
    }

    #[test]
    fn frame_round_trip() {
        let frame = RecordFrame::new(FrameKind::EventJson, 1024).unwrap();
        let bytes = frame.to_bytes();
        let restored = RecordFrame::from_bytes(&bytes).unwrap();
        assert_eq!(frame.kind.to_byte(), restored.kind.to_byte());
        assert_eq!(frame.len, restored.len);
    }

    #[test]
    fn frame_rejects_oversized_payload() {
        assert!(RecordFrame::new(FrameKind::EventJson, MAX_PAYLOAD_SIZE + 1).is_err());
    }

    #[test]
    fn frame_rejects_non_zero_reserved() {
        let mut bytes = RecordFrame::new(FrameKind::EventJson, 100)
            .unwrap()
            .to_bytes();
        bytes[1] = 0x01;
        assert!(RecordFrame::from_bytes(&bytes).is_err());
    }

    #[test]
    fn frame_kind_unknown() {
        let kind = FrameKind::from_byte(0xFF);
        assert_eq!(kind.to_byte(), 0xFF);
    }
}
