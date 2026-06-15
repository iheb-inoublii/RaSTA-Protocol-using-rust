// Abstract OS primitive / synchronization interface.
//
// The core stack does not depend on an operating system. Targets that need
// locking can provide an implementation from their platform adapter.

pub trait CriticalSection {
    fn enter(&mut self);
    fn exit(&mut self);
}
