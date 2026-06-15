use crate::platform::clock::Clock;

pub struct StdClock;

impl Clock for StdClock {
    fn now_ms(&self) -> u32 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u32
    }
}
