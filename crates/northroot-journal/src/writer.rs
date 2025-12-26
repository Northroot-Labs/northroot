//! Journal writer implementation.

use crate::errors::JournalError;
use crate::event::EventJson;
use crate::frame::{FrameKind, JournalHeader, RecordFrame};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::Path;

/// Options for journal writing.
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Whether to fsync after each append (default: false).
    pub sync: bool,
    /// Whether to create the file if it doesn't exist (default: true).
    pub create: bool,
    /// Whether to append to an existing file (default: true).
    pub append: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            sync: false,
            create: true,
            append: true,
        }
    }
}

/// Journal writer for append-only event storage.
pub struct JournalWriter {
    file: File,
    sync: bool,
    header_written: bool,
}

impl JournalWriter {
    /// Opens or creates a journal file for writing.
    pub fn open<P: AsRef<Path>>(path: P, options: WriteOptions) -> Result<Self, JournalError> {
        let file = OpenOptions::new()
            .create(options.create)
            .write(true)
            .read(true)
            .open(path)?;

        let mut writer = Self {
            file,
            sync: options.sync,
            header_written: false,
        };

        // Check if file is empty; if so, write header
        let metadata = writer.file.metadata()?;
        if metadata.len() == 0 {
            writer.write_header()?;
        } else if metadata.len() < JournalHeader::HEADER_SIZE as u64 {
            return Err(JournalError::FileNotEmpty);
        } else {
            // File exists and has at least header size; verify it's a valid journal
            let mut header_bytes = [0u8; JournalHeader::HEADER_SIZE];
            writer.file.seek(io::SeekFrom::Start(0))?;
            writer.file.read_exact(&mut header_bytes)?;
            JournalHeader::from_bytes(&header_bytes)?;
            writer.header_written = true;
            // Seek to end for appending
            if options.append {
                writer.file.seek(io::SeekFrom::End(0))?;
            } else {
                writer.file.seek(io::SeekFrom::Start(0))?;
                writer.file.set_len(JournalHeader::HEADER_SIZE as u64)?;
                writer
                    .file
                    .seek(io::SeekFrom::Start(JournalHeader::HEADER_SIZE as u64))?;
            }
        }

        Ok(writer)
    }

    fn write_header(&mut self) -> Result<(), JournalError> {
        let header = JournalHeader::new();
        let bytes = header.to_bytes();
        self.file.write_all(&bytes)?;
        self.file.flush()?;
        if self.sync {
            self.file.sync_all()?;
        }
        self.header_written = true;
        Ok(())
    }

    /// Appends an event JSON payload to the journal.
    pub fn append_event(&mut self, event: &EventJson) -> Result<(), JournalError> {
        let json_bytes = serde_json::to_vec(event)?;
        self.append_raw(FrameKind::EventJson, &json_bytes)
    }

    /// Appends a raw frame with the given kind and payload.
    pub fn append_raw(&mut self, kind: FrameKind, payload: &[u8]) -> Result<(), JournalError> {
        if !self.header_written {
            return Err(JournalError::InvalidHeader(
                "header not written".to_string(),
            ));
        }

        let frame = RecordFrame::new(kind, payload.len() as u32)?;
        let frame_bytes = frame.to_bytes();

        // Write frame header
        self.file.write_all(&frame_bytes)?;
        // Write payload
        self.file.write_all(payload)?;
        self.file.flush()?;

        if self.sync {
            self.file.sync_all()?;
        }

        Ok(())
    }

    /// Finishes writing and closes the file.
    pub fn finish(mut self) -> Result<(), JournalError> {
        self.file.flush()?;
        if self.sync {
            self.file.sync_all()?;
        }
        Ok(())
    }
}

impl Drop for JournalWriter {
    fn drop(&mut self) {
        let _ = self.file.flush();
        if self.sync {
            let _ = self.file.sync_all();
        }
    }
}
