use core::time::Duration;

use crate::{ClockStretching, Rate, Repeatability};

/// Minimum wait time between two commands, in milliseconds.
static MIN_WAIT_BETWEEN_COMMAND_MS: usize = 1;

/// Enumeration of all possible commands.
#[derive(Debug, Copy, Clone)]
pub(crate) enum Command {
    /// Execute a single measurement.
    SingleShot(ClockStretching, Repeatability),

    /// Initiate periodic data acquisition.
    Periodic(Rate, Repeatability),

    /// Retrieve data from a periodic measurement.
    FetchData,

    /// Initiate periodic data acquisition with Accelerated Response
    /// Time (ART).
    PeriodicWithART,

    /// Halt periodic data acquisition.
    Break,

    /// Perform a soft reset to bring the system to a predetermined
    /// state without disconnecting the power supply.
    SoftReset,

    /// Enable the heater.
    HeaterEnable,

    /// Disable the heater.
    HeaterDisable,

    /// Read the status register.
    ///
    /// # See also
    /// - [`sensirion_sht3x::Status`]
    StatusFetch,

    /// Clear specific flags (Bit 15, 11, 10, 4) in the status
    /// register, setting them to zero.
    StatusClear,
}

impl From<Command> for u16 {
    fn from(value: Command) -> Self {
        match value {
            Command::SingleShot(clock_stretching, repeatability) => match clock_stretching {
                ClockStretching::Enabled => match repeatability {
                    Repeatability::Low => 0x2c06,
                    Repeatability::Medium => 0x2c0d,
                    Repeatability::High => 0x2c10,
                },
                ClockStretching::Disabled => match repeatability {
                    Repeatability::Low => 0x2400,
                    Repeatability::Medium => 0x240b,
                    Repeatability::High => 0x2416,
                },
            },
            Command::Periodic(rate, repeatability) => match rate {
                Rate::R0_5 => match repeatability {
                    Repeatability::Low => 0x2032,
                    Repeatability::Medium => 0x2024,
                    Repeatability::High => 0x202f,
                },
                Rate::R1 => match repeatability {
                    Repeatability::Low => 0x2130,
                    Repeatability::Medium => 0x2126,
                    Repeatability::High => 0x212d,
                },
                Rate::R2 => match repeatability {
                    Repeatability::Low => 0x2236,
                    Repeatability::Medium => 0x2220,
                    Repeatability::High => 0x222b,
                },
                Rate::R4 => match repeatability {
                    Repeatability::Low => 0x2334,
                    Repeatability::Medium => 0x2322,
                    Repeatability::High => 0x2329,
                },
                Rate::R10 => match repeatability {
                    Repeatability::Low => 0x2737,
                    Repeatability::Medium => 0x2721,
                    Repeatability::High => 0x272a,
                },
            },
            Command::FetchData => 0xe000,
            Command::PeriodicWithART => 0x2b32,
            Command::Break => 0x3093,
            Command::SoftReset => 0x30a2,
            Command::HeaterEnable => 0x306d,
            Command::HeaterDisable => 0x3066,
            Command::StatusFetch => 0xf32d,
            Command::StatusClear => 0x3041,
        }
    }
}

impl From<Command> for Duration {
    fn from(value: Command) -> Self {
        match value {
            Command::SingleShot(_, repeatability) => match repeatability {
                Repeatability::Low => Duration::from_millis(4),
                Repeatability::Medium => Duration::from_millis(6),
                Repeatability::High => Duration::from_millis(15),
            },
            Command::SoftReset => Duration::from_micros(1500),
            _ => Duration::from_millis(
                MIN_WAIT_BETWEEN_COMMAND_MS
                    .try_into()
                    .expect("Failed to use MIN_WAIT_BETWEEN_COMMAND_MS to create Duration."),
            ),
        }
    }
}
