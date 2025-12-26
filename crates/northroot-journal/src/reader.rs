//! Journal reader implementation.

use crate::errors::JournalError;
use crate::event::EventJson;
use crate::frame::{FrameKind, JournalHeader, RecordFrame};
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;

/// Read mode for handling truncation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    /// Strict mode: truncated frames are errors.
    Strict,
    /// Permissive mode: truncation is treated as end-of-file.
    Permissive,
}

/// Journal reader for reading events from a journal file.
pub struct JournalReader {
    file: File,
    mode: ReadMode,
    position: u64,
}

impl JournalReader {
    /// Returns the current read position in the file.
    pub fn position(&self) -> u64 {
        self.position
    }
}

impl JournalReader {
    /// Opens a journal file for reading.
    pub fn open<P: AsRef<Path>>(path: P, mode: ReadMode) -> Result<Self, JournalError> {
        let mut file = File::open(path)?;
        let _header = Self::read_header(&mut file)?;
        let position = JournalHeader::HEADER_SIZE as u64;

        Ok(Self {
            file,
            mode,
            position,
        })
    }

    fn read_header(file: &mut File) -> Result<JournalHeader, JournalError> {
        file.seek(io::SeekFrom::Start(0))?;
        let mut header_bytes = [0u8; JournalHeader::HEADER_SIZE];
        file.read_exact(&mut header_bytes)?;
        JournalHeader::from_bytes(&header_bytes)
    }

    /// Reads the next frame from the journal.
    ///
    /// Returns `Ok(None)` when end-of-file is reached (or truncation in permissive mode).
    pub fn read_frame(&mut self) -> Result<Option<(FrameKind, Vec<u8>)>, JournalError> {
        self.file.seek(io::SeekFrom::Start(self.position))?;

        // Check if we're at EOF before trying to read
        let file_size = self.file.metadata()?.len();
        if self.position >= file_size {
            return Ok(None);
        }

        // Read frame header
        let mut frame_header_bytes = [0u8; RecordFrame::FRAME_HEADER_SIZE];
        match self.file.read_exact(&mut frame_header_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                if self.mode == ReadMode::Permissive {
                    return Ok(None);
                }
                return Err(JournalError::TruncatedFrame {
                    offset: self.position,
                });
            }
            Err(e) => return Err(e.into()),
        }

        let frame = RecordFrame::from_bytes(&frame_header_bytes).map_err(|e| match e {
            JournalError::InvalidFrame { offset: _, reason } => JournalError::InvalidFrame {
                offset: self.position,
                reason,
            },
            other => other,
        })?;

        self.position += RecordFrame::FRAME_HEADER_SIZE as u64;

        // Read payload
        let mut payload = vec![0u8; frame.len as usize];
        match self.file.read_exact(&mut payload) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                if self.mode == ReadMode::Permissive {
                    return Ok(None);
                }
                return Err(JournalError::TruncatedFrame {
                    offset: self.position,
                });
            }
            Err(e) => return Err(e.into()),
        }

        self.position += frame.len as u64;

        Ok(Some((frame.kind, payload)))
    }

    /// Reads the next event JSON from the journal.
    ///
    /// Skips unknown frame kinds and returns `Ok(None)` at end-of-file.
    pub fn read_event(&mut self) -> Result<Option<EventJson>, JournalError> {
        loop {
            match self.read_frame()? {
                None => return Ok(None),
                Some((FrameKind::EventJson, payload)) => {
                    // Validate UTF-8
                    let utf8_str = std::str::from_utf8(&payload)?;
                    // Parse JSON
                    let json: EventJson =
                        serde_json::from_str(utf8_str).map_err(JournalError::JsonParse)?;
                    return Ok(Some(json));
                }
                Some((FrameKind::Unknown(_), _)) => {
                    // Skip unknown frame kinds
                    continue;
                }
            }
        }
    }
}
