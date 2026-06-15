use crate::core::connection::{ConnectionError, RastaConfig, RastaConnection};
use crate::core::connection_state_machine::RastaState;
use crate::platform::clock::Clock;
use crate::platform::timer::Timer;
use crate::platform::transport::Transport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Down,
    Opening,
    Up,
    Retransmission,
    Closing,
}

impl From<RastaState> for ConnectionStatus {
    fn from(state: RastaState) -> Self {
        match state {
            RastaState::Down => Self::Down,
            RastaState::Start => Self::Opening,
            RastaState::Up => Self::Up,
            RastaState::Retransmission => Self::Retransmission,
            RastaState::Closed => Self::Closing,
        }
    }
}

pub struct RastaService<T1: Transport, T2: Transport, TimerCtx: Timer, C: Clock> {
    connection: RastaConnection<T1, T2, TimerCtx, C>,
}

pub type RastaApi<T1, T2, TimerCtx, C> = RastaService<T1, T2, TimerCtx, C>;

impl<T1: Transport, T2: Transport, TimerCtx: Timer, C: Clock> RastaService<T1, T2, TimerCtx, C> {
    pub fn new(
        transport_a: T1,
        transport_b: T2,
        timer: TimerCtx,
        clock: C,
        config: RastaConfig,
    ) -> Self {
        Self {
            connection: RastaConnection::new(transport_a, transport_b, timer, clock, config),
        }
    }

    pub fn open_connection(&mut self) -> Result<(), ConnectionError> {
        self.connection.connect()
    }

    pub fn close_connection(&mut self) -> Result<(), ConnectionError> {
        self.connection.disconnect()
    }

    pub fn send_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        if self.connection.state_machine.current_state != RastaState::Up {
            return Err(ConnectionError::StateTransitionInvalid);
        }
        self.connection
            .send_packet(crate::core::pdu::PacketType::Data, data)
    }

    pub fn poll(&mut self) -> Result<(), ConnectionError> {
        self.connection.process()
    }

    pub fn receive_data(&mut self, output: &mut [u8]) -> Result<usize, ConnectionError> {
        self.connection.receive_data(output)
    }

    pub fn has_received_data(&self) -> bool {
        self.connection.has_received_data()
    }

    pub fn status(&self) -> ConnectionStatus {
        self.connection.state_machine.current_state.into()
    }
}
