#[cfg(test)]
mod tests {
    use crate::core::connection::{RastaConfig, RastaConnection};
    use crate::core::connection_state_machine::{RastaState, StateMachine};
    use crate::core::pdu::{Packet, PacketType};
    use crate::core::redundancy_management::{RedundancyConfig, RedundancyLayer};
    use crate::core::retransmission::RetransmissionBuffer;
    use crate::core::safety_code::{Md4, SafetyCodeConfig};
    use crate::core::sequencing::{SequenceHandler, SequenceResult};
    use crate::core::time_supervision::{TimeSupervisionError, TimeSupervisor};
    use crate::platform::clock::Clock;
    use crate::platform::timer::Timer;
    use crate::platform::transport::{Transport, TransportError};

    struct MockClock {
        time: u32,
    }
    impl Clock for MockClock {
        fn now_ms(&self) -> u32 {
            self.time
        }
    }

    struct MockTimer {
        end_time: u32,
        running: bool,
    }
    impl Timer for MockTimer {
        fn start(&mut self, duration_ms: u32) {
            self.end_time = duration_ms; // Simplified for test
            self.running = true;
        }
        fn expired(&self) -> bool {
            self.running
        } // Simplified
        fn stop(&mut self) {
            self.running = false;
        }
    }

    #[derive(Clone, Copy)]
    struct SimpleMockTransport {
        receive_data: [u8; 512],
        receive_len: usize,
        sent: [u8; 512],
        sent_len: usize,
    }

    impl SimpleMockTransport {
        fn empty() -> Self {
            Self {
                receive_data: [0; 512],
                receive_len: 0,
                sent: [0; 512],
                sent_len: 0,
            }
        }

        fn with_receive(data: &[u8]) -> Self {
            let mut transport = Self::empty();
            transport.receive_data[..data.len()].copy_from_slice(data);
            transport.receive_len = data.len();
            transport
        }
    }

    impl Transport for SimpleMockTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            if data.len() > self.sent.len() {
                return Err(TransportError::BufferTooSmall);
            }
            self.sent[..data.len()].copy_from_slice(data);
            self.sent_len = data.len();
            Ok(())
        }
        fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            if self.receive_len == 0 {
                return Ok(0);
            }
            if buffer.len() < self.receive_len {
                return Err(TransportError::BufferTooSmall);
            }
            buffer[..self.receive_len].copy_from_slice(&self.receive_data[..self.receive_len]);
            let len = self.receive_len;
            self.receive_len = 0;
            Ok(len)
        }
    }

    fn config(sender_id: u32, remote_id: u32) -> RastaConfig {
        RastaConfig {
            sender_id,
            remote_id,
            safety_code: SafetyCodeConfig::default(),
            redundancy: RedundancyConfig::default(),
            t_max: 2000,
            initial_seq: 0,
            heartbeat_interval_ms: 500,
            n_send_max: 16,
        }
    }

    #[test]
    fn test_packet_serialization() {
        let safety = SafetyCodeConfig::default();
        let packet = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 10,
            confirmed_sequence_number: 5,
            timestamp: 1000,
            confirmed_timestamp: 900,
            packet_type: PacketType::Data,
            payload: [0u8; 256],
            payload_len: 4,
        };
        // Set some dummy payload
        let mut p = packet;
        p.payload[0] = 0xAA;
        p.payload[1] = 0xBB;
        p.payload[2] = 0xCC;
        p.payload[3] = 0xDD;

        let mut buffer = [0u8; 512];
        let size = p
            .serialize(&mut buffer, &safety)
            .expect("Serialization failed");

        assert_eq!(u16::from_le_bytes([buffer[2], buffer[3]]), 6240);

        let parsed = Packet::parse(&buffer[..size], &safety).expect("Parsing failed");
        assert_eq!(parsed.receiver_id, 1);
        assert_eq!(parsed.sender_id, 2);
        assert_eq!(parsed.sequence_number, 10);
        assert_eq!(parsed.payload_len, 4);
        assert_eq!(parsed.payload[0], 0xAA);
    }

    #[test]
    fn test_state_machine_transitions() {
        let mut sm = StateMachine::new();
        assert_eq!(sm.current_state, RastaState::Down);

        // Valid transition
        assert!(sm.transition(RastaState::Start));
        assert_eq!(sm.current_state, RastaState::Start);

        // Invalid transition: Down -> Up (must go through Start)
        let mut sm2 = StateMachine::new();
        assert!(!sm2.transition(RastaState::Up));
        assert_eq!(sm2.current_state, RastaState::Down);
    }

    #[test]
    fn test_sequence_handler() {
        let mut sh = SequenceHandler::new();
        assert_eq!(sh.next_tx(), 0);
        assert_eq!(sh.next_tx(), 1);

        // Receive 0 (expecting 0)
        assert_eq!(sh.validate_rx(0), SequenceResult::Ok);
        // Receive 1 (expecting 1)
        assert_eq!(sh.validate_rx(1), SequenceResult::Ok);
        // Receive 3 (Gap)
        match sh.validate_rx(3) {
            SequenceResult::Gap(expected) => assert_eq!(expected, 2),
            _ => panic!("Expected Gap"),
        }
    }

    #[test]
    fn test_md4_known_vectors() {
        let empty_digest = Md4::new().finalize();
        assert_eq!(
            empty_digest,
            [
                0x31, 0xd6, 0xcf, 0xe0, 0xd1, 0x6a, 0xe9, 0x31, 0xb7, 0x3c, 0x59, 0xd7, 0xe0, 0xc0,
                0x89, 0xc0,
            ]
        );

        let mut md4 = Md4::new();
        md4.update(b"abc");
        assert_eq!(
            md4.finalize(),
            [
                0xa4, 0x48, 0x01, 0x7a, 0xaf, 0x21, 0xd8, 0x52, 0x5f, 0xc1, 0x0a, 0xe8, 0x7a, 0xa6,
                0x72, 0x9d,
            ]
        );
    }

    #[test]
    fn test_time_supervision() {
        let supervisor = TimeSupervisor::new(2000);

        assert!(supervisor.validate(3000, 1500).is_ok());
        assert_eq!(
            supervisor.validate(3000, 900),
            Err(TimeSupervisionError::TimestampTooOld)
        );
        assert_eq!(
            supervisor.validate(3000, 3200),
            Err(TimeSupervisionError::TimestampTooFarInFuture)
        );
    }

    #[test]
    fn test_retransmission_buffer() {
        let mut rb = RetransmissionBuffer::new();
        let packet = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 100,
            confirmed_sequence_number: 0,
            timestamp: 0,
            confirmed_timestamp: 0,
            packet_type: PacketType::Data,
            payload: [0u8; 256],
            payload_len: 0,
        };

        assert!(rb.store(packet));
        assert_eq!(rb.count(), 1);

        let retrieved = rb.get_packet(100).expect("Packet not found");
        assert_eq!(retrieved.sequence_number, 100);

        rb.clear_up_to(100);
        assert_eq!(rb.count(), 0);
    }

    #[test]
    fn test_connection_handshake_start() {
        let clock = MockClock { time: 0 };
        let timer = MockTimer {
            end_time: 0,
            running: false,
        };
        let config = RastaConfig { ..config(123, 0) };
        let mut conn = RastaConnection::new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            timer,
            clock,
            config,
        );

        assert_eq!(conn.state_machine.current_state, RastaState::Down);
        conn.connect().expect("Connect failed");
        assert_eq!(conn.state_machine.current_state, RastaState::Start);
    }

    #[test]
    fn test_application_receive_queue() {
        let clock = MockClock { time: 0 };
        let timer = MockTimer {
            end_time: 0,
            running: false,
        };
        let mut conn = RastaConnection::new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            timer,
            clock,
            config(1, 2),
        );

        conn.transition(RastaState::Start).unwrap();
        conn.sequence.accept_initial_rx(99);
        conn.transition(RastaState::Up).unwrap();

        let mut packet = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 100,
            confirmed_sequence_number: 0,
            timestamp: 0,
            confirmed_timestamp: 0,
            packet_type: PacketType::Data,
            payload: [0; 256],
            payload_len: 5,
        };
        packet.payload[..5].copy_from_slice(b"hello");

        let mut wire = [0u8; 512];
        let len = packet
            .serialize(&mut wire, &SafetyCodeConfig::default())
            .unwrap();
        let mut rl_frame = [0u8; 520];
        let total = len + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        rl_frame[..2].copy_from_slice(&(total as u16).to_le_bytes());
        rl_frame[4..8].copy_from_slice(&0u32.to_le_bytes());
        rl_frame[8..total].copy_from_slice(&wire[..len]);

        conn.redundancy = RedundancyLayer::new(
            SimpleMockTransport::with_receive(&rl_frame[..total]),
            SimpleMockTransport::empty(),
        );
        conn.process().unwrap();

        let mut out = [0u8; 16];
        let received = conn.receive_data(&mut out).unwrap();
        assert_eq!(&out[..received], b"hello");
    }

    #[test]
    fn test_redundancy_discards_duplicate_channel_copy() {
        let payload = b"safe-pdu";
        let total = payload.len()
            + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        let mut frame = [0u8; 520];
        frame[..2].copy_from_slice(&(total as u16).to_le_bytes());
        frame[4..8].copy_from_slice(&0u32.to_le_bytes());
        frame[8..total].copy_from_slice(payload);

        let mut redundancy = RedundancyLayer::new(
            SimpleMockTransport::with_receive(&frame[..total]),
            SimpleMockTransport::with_receive(&frame[..total]),
        );

        let mut out = [0u8; 32];
        let len = redundancy.receive(&mut out).unwrap();
        assert_eq!(&out[..len], payload);

        let second = redundancy.receive(&mut out).unwrap();
        assert_eq!(second, 0);
    }
}
