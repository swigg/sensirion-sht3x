#![doc = include_str!("../README.md")]
#![deny(unsafe_code, missing_docs)]
#![cfg_attr(not(test), no_std)]
#[cfg(feature = "log")]
extern crate alloc;

use bitflags::bitflags;
use core::{fmt::Display, u16};

///
pub mod blocking;

///
#[cfg(feature = "embedded-hal-async")]
pub mod asynchronous;

#[doc(hidden)]
pub mod command;

///
pub mod error;

use bytes::Buf;
#[cfg(test)]
use bytes::{BufMut, BytesMut};
use embedded_hal::i2c::SevenBitAddress;
use measurements::{Humidity, Temperature};

#[derive(Debug, Clone, Copy)]
/// The sensor’s I2C address
pub enum AddressPin {
    /// The default address is 0x44
    Low,

    /// The optional secondary address is 0x45
    High,
}

impl Into<SevenBitAddress> for AddressPin {
    fn into(self) -> SevenBitAddress {
        match self {
            AddressPin::Low => 0x44,
            AddressPin::High => 0x45,
        }
    }
}

impl Default for AddressPin {
    fn default() -> Self {
        Self::Low
    }
}

/// When a command with clock stretching is issued, the sensor acknowledges
/// with an ACK upon receiving a read header and then pulls down the SCL
/// line. The SCL line remains pulled down until the measurement is
/// completed. Once finished, the sensor releases the SCL line and
/// transmits the measurement results.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ClockStretching {
    /// Enable clock stretching
    Enabled,

    /// Disable clock stretching
    Disabled,
}

/// The specified repeatability corresponds to three times the standard
/// deviation (3σ) of multiple consecutive measurements, taken at the
/// specified repeatability under constant ambient conditions. It measures
/// the noise present on the sensor's physical output. Various measurement
/// modes provide options for high, medium, or low repeatability.
#[derive(Debug, Clone, Copy)]
pub enum Repeatability {
    /// 3 times the standard deviation: 0.15 °C, 0.21 %RH
    Low,
    /// 3 times the standard deviation: 0.08 °C, 0.15 %RH
    Medium,
    /// 3 times the standard deviation: 0.04 °C, 0.08 %RH
    High,
}

/// Represents the data acquisition rate settings for periodic measurements.
#[derive(Debug, Clone, Copy)]
pub enum Rate {
    /// Rate: 0.5 measurements per second.
    R0_5,
    /// Rate: 1 measurement per second.
    R1,
    /// Rate: 2 measurements per second.
    R2,
    /// Rate: 4 measurements per second.
    R4,
    /// Rate: 10 measurements per second.
    R10,
}

bitflags! {
    /// The status register provides operational status details of the
    /// sensor. It indicates the heater status, alert mode, and execution
    /// status of the last command and write sequence.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Status: u16 {
        /// Indicates an incorrect checksum in the last write operation.
        const WRITE_DATA_CHECKSUM = 1 << 0;

        /// Indicates that the last command was not executed successfully.
        const COMMAND = 1 << 1;

        /// Indicates a system reset was detected.
        const RESET = 1 << 4;

        /// Indicates a temperature tracking alert has occurred.
        const T_TRACKING_ALERT = 1 << 10;

        /// Indicates a relative humidity tracking alert has occurred.
        const RH_TRACKING_ALERT = 1 << 11;

        /// Indicates whether the heater is currently active.
        const HEATER = 1 << 13;

        /// Indicates if any alert is pending.
        const ALERT_PENDING = 1 << 15;
    }
}

impl Status {
    /// Checks if the heater is enabled.
    pub fn heater_enabled(&self) -> bool {
        self.intersects(Status::HEATER)
    }

    /// Determines if there is a write data checksum error.
    pub fn write_data_checksum_error(&self) -> bool {
        self.intersects(Status::WRITE_DATA_CHECKSUM)
    }

    /// Indicates if the last command execution was unsuccessful.
    pub fn command_execution_failed(&self) -> bool {
        self.intersects(Status::COMMAND)
    }

    /// Checks if a system reset was detected.
    pub fn system_reset_detected(&self) -> bool {
        self.intersects(Status::RESET)
    }

    /// Verifies if a temperature tracking alert has occurred.
    pub fn temperature_tracking_alert_occurred(&self) -> bool {
        self.intersects(Status::T_TRACKING_ALERT)
    }

    /// Verifies if a relative humidity tracking alert has occurred.
    pub fn relative_humidity_tracking_alert_occurred(&self) -> bool {
        self.intersects(Status::RH_TRACKING_ALERT)
    }

    /// Checks if any alert is pending.
    pub fn alert_pending(&self) -> bool {
        self.intersects(Status::ALERT_PENDING)
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[cfg(test)]
impl From<Status> for Vec<u8> {
    fn from(value: Status) -> Self {
        let mut buffer = BytesMut::with_capacity(3);
        buffer.put_u16(value.bits());
        buffer.put_u8(sensirion_i2c::crc8::calculate(&value.bits().to_be_bytes()));
        buffer.to_vec()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents a sensor measurement.
pub struct Measurement {
    /// The temperature reported by the sensor.
    pub temperature: Temperature,
    
    /// The relative humidity reported by the sensor.
    pub relative_humidity: Humidity,
}

impl<T: AsRef<[u8]>> From<T> for Measurement {
    fn from(value: T) -> Self {
        Measurement {
            temperature: temperature_from_raw(value.as_ref().get(0..2).unwrap().get_u16()),
            relative_humidity: relative_humidity_from_raw(
                value.as_ref().get(3..5).unwrap().get_u16(),
            ),
        }
    }
}

#[cfg(test)]
impl From<Measurement> for Vec<u8> {
    fn from(value: Measurement) -> Self {
        let mut buffer = BytesMut::with_capacity(6);

        let relative_humidity = ((value.relative_humidity.as_percent() * 65535.0) / 100.0) as u16;
        buffer.put_u16(relative_humidity);
        buffer.put_u8(sensirion_i2c::crc8::calculate(
            &relative_humidity.to_be_bytes(),
        ));

        let temperature =
            (((value.temperature.as_celsius() + 45.0) * u16::MAX as f64) / 175.0) as u16;
        buffer.put_u16(temperature);
        buffer.put_u8(sensirion_i2c::crc8::calculate(&temperature.to_be_bytes()));

        buffer.to_vec()
    }
}

/// Converts a raw sensor measurement into a [`measurement::Temperature`].
///
/// # Arguments
///
/// * `raw` - The raw temperature measurement from the sensor.
///
/// # Returns
///
/// A `Temperature` object representing the converted temperature.
fn temperature_from_raw(raw: u16) -> Temperature {
    Temperature::from_celsius(-45.0 + 175.0 * raw as f64 / u16::MAX as f64)
}

/// Converts a raw sensor measurement into a [`measurement::Humidity`].
///
/// # Arguments
///
/// * `raw` - The raw humidity measurement from the sensor.
///
/// # Returns
///
/// A `Humidity` object representing the converted relative humidity.
fn relative_humidity_from_raw(raw: u16) -> Humidity {
    Humidity::from_percent(raw as f64 / u16::MAX as f64 * 100.0)
}

#[cfg(test)]
mod tests {
    use ctor::ctor;
    use env_logger::{Builder, Env};

    use crate::{relative_humidity_from_raw, temperature_from_raw};

    #[ctor]
    fn init_logger() {
        let _ = Builder::from_env(Env::default()).init();
    }

    #[test]
    fn test_temperature_from_raw() {
        let temperature = temperature_from_raw(35576);
        assert!(
            (49.9..50.0).contains(&temperature.as_celsius()),
            "Expected 50ºC"
        );
    }

    #[test]
    fn test_humidity_from_raw() {
        let relative_humidity = relative_humidity_from_raw(32767);
        assert!(
            (49.9..50.0).contains(&relative_humidity.as_percent()),
            "Expected 50% humidity"
        );
    }

    #[test]
    fn test_measurement_from_u8() {}
}
