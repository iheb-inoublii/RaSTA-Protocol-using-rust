pub mod embedded_ethernet;
pub mod test;

#[cfg(feature = "std")]
pub mod socket_transport;
#[cfg(feature = "std")]
pub mod standard_clock;
#[cfg(feature = "std")]
pub mod standard_timer;

#[cfg(all(feature = "std", target_os = "linux"))]
pub mod linux;
#[cfg(all(feature = "std", target_os = "windows"))]
pub mod windows;
