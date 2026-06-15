pub mod service_interface;

#[deprecated(note = "renamed to service_interface")]
pub mod api {
    pub use super::service_interface::{ConnectionStatus, RastaApi, RastaService};
}
