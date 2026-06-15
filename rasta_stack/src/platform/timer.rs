// Platform-independent timer interface.
//
// The protocol core depends on this trait only. OS timers, hardware timers,
// and test timers are supplied by adapters.

pub trait Timer {
    fn start(&mut self, duration_ms: u32);
    fn expired(&self) -> bool;
    fn stop(&mut self);
}
