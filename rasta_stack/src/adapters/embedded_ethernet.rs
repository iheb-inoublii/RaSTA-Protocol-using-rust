use crate::platform::transport::{Transport, TransportError};

pub trait EmbeddedEthernetDriver {
    fn send_frame(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn receive_frame(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}

pub struct EmbeddedEthernetAdapter<D: EmbeddedEthernetDriver> {
    driver: D,
}

impl<D: EmbeddedEthernetDriver> EmbeddedEthernetAdapter<D> {
    pub fn new(driver: D) -> Self {
        Self { driver }
    }

    pub fn driver(&self) -> &D {
        &self.driver
    }

    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }
}

impl<D: EmbeddedEthernetDriver> Transport for EmbeddedEthernetAdapter<D> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.driver.send_frame(data)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        self.driver.receive_frame(buffer)
    }
}
