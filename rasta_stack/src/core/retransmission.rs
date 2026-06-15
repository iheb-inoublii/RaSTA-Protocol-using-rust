// Retransmission logic with ordered retrieval
// RaSTA requires retransmitted packets to be in strict sequence.

use crate::core::pdu::Packet;

pub struct RetransmissionBuffer {
    pub packets: [Option<Packet>; 16],
    // Oldest retained packet. When the fixed buffer is full, this packet is
    // replaced by the new one so the stack keeps moving without allocation.
    oldest_seq: Option<u32>,
}

impl Default for RetransmissionBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RetransmissionBuffer {
    pub fn new() -> Self {
        const NONE_PACKET: Option<Packet> = None;
        RetransmissionBuffer {
            packets: [NONE_PACKET; 16],
            oldest_seq: None,
        }
    }

    pub fn store(&mut self, packet: Packet) -> bool {
        for slot in self.packets.iter_mut() {
            if slot.is_none() {
                let seq = packet.sequence_number;
                *slot = Some(packet);
                self.update_oldest_after_insert(seq);
                return true;
            }
        }

        if let Some(oldest) = self.oldest_seq {
            for slot in self.packets.iter_mut() {
                if slot
                    .as_ref()
                    .filter(|p| p.sequence_number == oldest)
                    .is_some()
                {
                    *slot = Some(packet);
                    self.recalculate_oldest();
                    return true;
                }
            }
        }
        false
    }

    pub fn clear_up_to(&mut self, confirmed_seq: u32) {
        // Remove packets that the other side acknowledged (seq <= confirmed_seq)
        for slot in self.packets.iter_mut() {
            if slot
                .as_mut()
                .filter(|p| confirmed_seq.wrapping_sub(p.sequence_number) < 0x80000000)
                .is_some()
            {
                *slot = None;
            }
        }

        self.recalculate_oldest();
    }

    fn update_oldest_after_insert(&mut self, seq: u32) {
        match self.oldest_seq {
            None => self.oldest_seq = Some(seq),
            Some(old) if seq.wrapping_sub(old) > 0x80000000 => self.oldest_seq = Some(seq),
            Some(_) => {}
        }
    }

    fn recalculate_oldest(&mut self) {
        self.oldest_seq = None;
        let mut oldest = None;
        for p in self.packets.iter().flatten() {
            match oldest {
                None => oldest = Some(p.sequence_number),
                Some(old) if p.sequence_number.wrapping_sub(old) > 0x80000000 => {
                    oldest = Some(p.sequence_number)
                }
                Some(_) => {}
            }
        }
        self.oldest_seq = oldest;
    }

    pub fn get_packet(&self, seq: u32) -> Option<&Packet> {
        self.packets
            .iter()
            .flatten()
            .find(|p| p.sequence_number == seq)
    }

    pub fn count(&self) -> usize {
        self.packets.iter().flatten().count()
    }
}
