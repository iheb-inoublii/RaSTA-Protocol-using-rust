// Platform-independent time source interface.
//
// Core RaSTA logic only needs a monotonically increasing millisecond value.
// Concrete implementations live in adapters so embedded, desktop, and test
// targets can provide time in their own way.
pub trait Clock {
    fn now_ms(&self) -> u32;
}
