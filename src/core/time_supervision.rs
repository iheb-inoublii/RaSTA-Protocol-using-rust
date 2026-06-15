// Time / timestamp supervision for platform-independent RaSTA core logic.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSupervisionError {
    TimestampTooOld,
    TimestampTooFarInFuture,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeSupervisor {
    pub t_max_ms: u32,
    pub future_tolerance_ms: u32,
}

impl TimeSupervisor {
    pub const DEFAULT_FUTURE_TOLERANCE_MS: u32 = 100;

    pub fn new(t_max_ms: u32) -> Self {
        Self {
            t_max_ms,
            future_tolerance_ms: Self::DEFAULT_FUTURE_TOLERANCE_MS,
        }
    }

    pub fn validate(
        &self,
        local_now_ms: u32,
        remote_timestamp_ms: u32,
    ) -> Result<(), TimeSupervisionError> {
        let age = local_now_ms.wrapping_sub(remote_timestamp_ms);

        if age < 0x8000_0000 {
            if age > self.t_max_ms {
                return Err(TimeSupervisionError::TimestampTooOld);
            }
        } else {
            let future_offset = remote_timestamp_ms.wrapping_sub(local_now_ms);
            if future_offset > self.future_tolerance_ms {
                return Err(TimeSupervisionError::TimestampTooFarInFuture);
            }
        }

        Ok(())
    }
}
