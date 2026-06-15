use crate::platform::timer::Timer;

pub struct StdTimer {
    start_time: Option<std::time::Instant>,
    duration: std::time::Duration,
}

impl Default for StdTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl StdTimer {
    pub fn new() -> Self {
        Self {
            start_time: None,
            duration: std::time::Duration::from_millis(0),
        }
    }
}

impl Timer for StdTimer {
    fn start(&mut self, duration_ms: u32) {
        self.start_time = Some(std::time::Instant::now());
        self.duration = std::time::Duration::from_millis(duration_ms as u64);
    }

    fn expired(&self) -> bool {
        if let Some(start) = self.start_time {
            start.elapsed() >= self.duration
        } else {
            false
        }
    }

    fn stop(&mut self) {
        self.start_time = None;
    }
}
