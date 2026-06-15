// Redundancy management for the Redundancy Layer.
//
// This layer is intentionally independent from UDP/TCP/Ethernet. It receives
// two objects that implement the portable Transport trait and exposes one
// logical channel to the Safety and Retransmission Layer above it.

use crate::platform::transport::{Transport, TransportError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RedundancyCheckCode {
    None,
    Crc16,
    Crc32,
}

#[derive(Clone, Copy, Debug)]
pub struct RedundancyConfig {
    pub check_code: RedundancyCheckCode,
}

impl Default for RedundancyConfig {
    fn default() -> Self {
        Self {
            check_code: RedundancyCheckCode::None,
        }
    }
}

impl RedundancyConfig {
    fn check_code_len(&self) -> usize {
        match self.check_code {
            RedundancyCheckCode::None => 0,
            RedundancyCheckCode::Crc16 => 2,
            RedundancyCheckCode::Crc32 => 4,
        }
    }
}

pub struct RedundancyLayer<T1: Transport, T2: Transport> {
    transport_a: T1,
    transport_b: T2,
    config: RedundancyConfig,
    tx_seq: u32,
    rx_seq: u32,
}

impl<T1: Transport, T2: Transport> RedundancyLayer<T1, T2> {
    pub const HEADER_SIZE: usize = 8;

    pub fn new(transport_a: T1, transport_b: T2) -> Self {
        Self::with_config(transport_a, transport_b, RedundancyConfig::default())
    }

    pub fn with_config(transport_a: T1, transport_b: T2, config: RedundancyConfig) -> Self {
        RedundancyLayer {
            transport_a,
            transport_b,
            config,
            tx_seq: 0,
            rx_seq: 0,
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let mut buffer = [0u8; 520];
        let total_len = data
            .len()
            .checked_add(Self::HEADER_SIZE)
            .and_then(|n| n.checked_add(self.config.check_code_len()))
            .ok_or(TransportError::BufferTooSmall)?;

        if total_len > buffer.len() {
            return Err(TransportError::BufferTooSmall);
        }

        buffer
            .get_mut(0..2)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&(total_len as u16).to_le_bytes());
        buffer
            .get_mut(2..4)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&0u16.to_le_bytes());
        buffer
            .get_mut(4..8)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&self.tx_seq.to_le_bytes());

        let dst = buffer
            .get_mut(8..8 + data.len())
            .ok_or(TransportError::BufferTooSmall)?;
        dst.copy_from_slice(data);

        self.write_check_code(&mut buffer, 8 + data.len(), total_len)?;

        self.tx_seq = self.tx_seq.wrapping_add(1);

        let res_a = self.transport_a.send(
            buffer
                .get(..total_len)
                .ok_or(TransportError::BufferTooSmall)?,
        );
        let res_b = self.transport_b.send(
            buffer
                .get(..total_len)
                .ok_or(TransportError::BufferTooSmall)?,
        );

        if res_a.is_err() && res_b.is_err() {
            return Err(TransportError::SendFailed);
        }
        Ok(())
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        let mut temp_buffer = [0u8; 520];
        let mut saw_error = false;

        for channel in 0..2 {
            let read_res = if channel == 0 {
                self.transport_a.receive(&mut temp_buffer)
            } else {
                self.transport_b.receive(&mut temp_buffer)
            };

            match read_res {
                Ok(0) => {}
                Ok(bytes_read) => {
                    if let Some(len) = self.accept_frame(&temp_buffer, bytes_read, buffer)? {
                        return Ok(len);
                    }
                }
                Err(TransportError::ReceiveFailed) => {
                    saw_error = true;
                }
                Err(e) => return Err(e),
            }
        }

        if saw_error {
            return Err(TransportError::ReceiveFailed);
        }
        Ok(0)
    }

    fn accept_frame(
        &mut self,
        frame: &[u8],
        bytes_read: usize,
        output: &mut [u8],
    ) -> Result<Option<usize>, TransportError> {
        let check_len = self.config.check_code_len();
        if bytes_read < Self::HEADER_SIZE + check_len {
            return Ok(None);
        }

        let declared_len = u16::from_le_bytes([
            *frame.first().ok_or(TransportError::BufferTooSmall)?,
            *frame.get(1).ok_or(TransportError::BufferTooSmall)?,
        ]) as usize;
        if declared_len != bytes_read {
            return Ok(None);
        }
        if !self.check_code_matches(frame, declared_len)? {
            return Ok(None);
        }

        let seq_bytes = frame.get(4..8).ok_or(TransportError::BufferTooSmall)?;
        let r_seq = u32::from_le_bytes([
            *seq_bytes.first().ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(2).ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(3).ok_or(TransportError::BufferTooSmall)?,
        ]);

        if r_seq.wrapping_sub(self.rx_seq) >= 0x80000000 {
            return Ok(None);
        }

        self.rx_seq = r_seq.wrapping_add(1);
        let payload_end = declared_len - check_len;
        let payload_len = payload_end - Self::HEADER_SIZE;
        if payload_len > output.len() {
            return Err(TransportError::BufferTooSmall);
        }
        let src = frame
            .get(Self::HEADER_SIZE..payload_end)
            .ok_or(TransportError::BufferTooSmall)?;
        let dst = output
            .get_mut(..payload_len)
            .ok_or(TransportError::BufferTooSmall)?;
        dst.copy_from_slice(src);
        Ok(Some(payload_len))
    }

    fn write_check_code(
        &self,
        frame: &mut [u8],
        check_start: usize,
        total_len: usize,
    ) -> Result<(), TransportError> {
        match self.config.check_code {
            RedundancyCheckCode::None => Ok(()),
            RedundancyCheckCode::Crc16 => {
                let crc = crc16(
                    frame
                        .get(..check_start)
                        .ok_or(TransportError::BufferTooSmall)?,
                );
                frame
                    .get_mut(check_start..total_len)
                    .ok_or(TransportError::BufferTooSmall)?
                    .copy_from_slice(&crc.to_le_bytes());
                Ok(())
            }
            RedundancyCheckCode::Crc32 => {
                let crc = crc32(
                    frame
                        .get(..check_start)
                        .ok_or(TransportError::BufferTooSmall)?,
                );
                frame
                    .get_mut(check_start..total_len)
                    .ok_or(TransportError::BufferTooSmall)?
                    .copy_from_slice(&crc.to_le_bytes());
                Ok(())
            }
        }
    }

    fn check_code_matches(&self, frame: &[u8], total_len: usize) -> Result<bool, TransportError> {
        let check_len = self.config.check_code_len();
        let check_start = total_len - check_len;
        match self.config.check_code {
            RedundancyCheckCode::None => Ok(true),
            RedundancyCheckCode::Crc16 => {
                let expected = crc16(
                    frame
                        .get(..check_start)
                        .ok_or(TransportError::BufferTooSmall)?,
                );
                let bytes = frame
                    .get(check_start..total_len)
                    .ok_or(TransportError::BufferTooSmall)?;
                let received = u16::from_le_bytes([
                    *bytes.first().ok_or(TransportError::BufferTooSmall)?,
                    *bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
                ]);
                Ok(received == expected)
            }
            RedundancyCheckCode::Crc32 => {
                let expected = crc32(
                    frame
                        .get(..check_start)
                        .ok_or(TransportError::BufferTooSmall)?,
                );
                let bytes = frame
                    .get(check_start..total_len)
                    .ok_or(TransportError::BufferTooSmall)?;
                let received = u32::from_le_bytes([
                    *bytes.first().ok_or(TransportError::BufferTooSmall)?,
                    *bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
                    *bytes.get(2).ok_or(TransportError::BufferTooSmall)?,
                    *bytes.get(3).ok_or(TransportError::BufferTooSmall)?,
                ]);
                Ok(received == expected)
            }
        }
    }
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xffffu16;
    for byte in data {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xedb8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
