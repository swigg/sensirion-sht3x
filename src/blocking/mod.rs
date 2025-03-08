use crate::{command::Command, error::Error, Measurement, Rate, Repeatability, Status};
use bytes::{Buf, BufMut, BytesMut};
use core::fmt::Debug;
use core::time::Duration;
use embedded_hal::i2c::{Operation, SevenBitAddress};

/// Represents a blocking driver for the SHT3x device.
#[derive(Debug, Default)]
pub struct Sht3x<I2C, A, D> {
    i2c: I2C,
    address: A,
    delay: D,
}

impl<I2C, D> Sht3x<I2C, SevenBitAddress, D>
where
    I2C: embedded_hal::i2c::I2c + Debug,
    D: embedded_hal::delay::DelayNs,
{
    /// Instantiate a new blocking SHT3x device driver.
    pub fn new(i2c: I2C, address: SevenBitAddress, delay: D) -> Self {
        Self {
            address: address.into(),
            i2c,
            delay,
        }
    }

    /// Perform a single-shot measurement with the given repeatability.
    pub fn measure_singleshot(
        &mut self,
        repeatability: Repeatability,
    ) -> Result<Measurement, Error<I2C::Error>> {
        let mut measurement = BytesMut::with_capacity(6);
        measurement.resize(6, 0);

        self.read_command(
            Command::SingleShot(crate::ClockStretching::Disabled, repeatability),
            &mut measurement,
        )
        .map(Measurement::from)
    }

    /// Perform a single-shot measurement with clock stretching.
    pub fn measure_singleshot_with_clock_stretching(
        &mut self,
        repeatability: Repeatability,
    ) -> Result<Measurement, Error<I2C::Error>> {
        let mut measurement = BytesMut::with_capacity(6);
        measurement.resize(6, 0);

        self.i2c
            .transaction(
                self.address,
                &mut [
                    Operation::Write(
                        &u16::from(Command::SingleShot(
                            crate::ClockStretching::Enabled,
                            repeatability,
                        ))
                        .to_be_bytes(),
                    ),
                    Operation::Read(&mut measurement),
                ],
            )
            .map(|_| Measurement::from(measurement))
            .map_err(Error::I2c)
    }

    /// Begin publishing periodic measurements.
    pub fn periodic_measurements_start(
        &mut self,
        repeatability: Repeatability,
        rate: Rate,
    ) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::Periodic(rate, repeatability))
    }

    /// Fetch the latest periodic measurement.
    pub fn periodic_measurement_fetch(&mut self) -> Result<Measurement, Error<I2C::Error>> {
        let mut measurement = BytesMut::with_capacity(6);
        measurement.resize(6, 0);

        self.read_command(Command::FetchData, &mut measurement)
            .map(Measurement::from)
    }

    /// Start periodic measurements using Accelerated Response Time (ART).
    pub fn periodic_measurement_start_with_art(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::PeriodicWithART)
    }

    /// Stop publishing periodic measurements.
    pub fn periodic_measurement_stop(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::Break)
    }

    /// Issue a soft reset to the device.
    pub fn soft_reset(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::SoftReset)
    }

    /// Enable the onboard heater.
    pub fn heater_enable(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::HeaterEnable)
    }

    /// Disable the onboard heater.
    pub fn heater_disable(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::HeaterDisable)
    }

    /// Read the contents of the status register.
    pub fn status(&mut self) -> Result<Status, Error<I2C::Error>> {
        let mut status = BytesMut::with_capacity(3);
        status.resize(3, 0);

        self.read_command(Command::Status, &mut status)
            .map(|status| Status::from_bits_retain(status.get(0..2).unwrap().get_u16()))
    }

    /// Clear the status register.
    pub fn status_clear(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::StatusClear)
    }

    fn read_command<'a>(
        &mut self,
        command: Command,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], Error<I2C::Error>> {
        self.read_command_with_args(command, None, buffer)
    }

    fn read_command_with_args<'a>(
        &mut self,
        command: Command,
        args: Option<&[u16]>,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], Error<I2C::Error>> {
        self.write_command_with_args(command, args)?;

        let delay_duration: Duration = command.into();
        #[cfg(any(debug_assertions, feature = "log"))]
        log::trace!(
            "Waiting {:?} after sending {{address: {:#x?}, command: {:?}[{}]}}",
            &delay_duration,
            self.address,
            command,
            u16::from(command).to_be_bytes().iter().map(|b| alloc::format!("{:#x?}", b)).collect::<alloc::vec::Vec<_>>().join(", "),
        );
        self.delay.delay_ns(delay_duration.as_nanos() as u32);

        sensirion_i2c::i2c::read_words_with_crc(&mut self.i2c, self.address, buffer)
            .map_err(Error::from)?;

        #[cfg(any(debug_assertions, feature = "log"))]
        log::trace!(
            "Read from bus {{address: {:#x?}, command: {:?}[{}], response: [{}]}}",
            self.address,
            command,
            u16::from(command).to_be_bytes().iter().map(|b| alloc::format!("{:#x?}", b)).collect::<alloc::vec::Vec<_>>().join(", "),
            &buffer[..].iter().map(|b| alloc::format!("{:#x?}", b)).collect::<alloc::vec::Vec<_>>().join(", "),
        );

        Ok(buffer)
    }

    fn write_command(&mut self, command: Command) -> Result<(), Error<I2C::Error>> {
        let command_code: u16 = command.into();

        #[cfg(any(debug_assertions, feature = "log"))]
        log::trace!(
            "Writing to bus {{address: {:#x?}, command: {:?}[{}]}}",
            self.address,
            command,
            u16::from(command).to_be_bytes().iter().map(|b| alloc::format!("{:#x?}", b)).collect::<alloc::vec::Vec<_>>().join(", "),
        );
        sensirion_i2c::i2c::write_command_u16(&mut self.i2c, self.address, command_code)
            .map_err(sensirion_i2c::i2c::Error::<I2C>::I2cWrite)?;
        Ok(())
    }

    fn write_command_with_args(
        &mut self,
        command: Command,
        args: Option<&[u16]>,
    ) -> Result<(), Error<I2C::Error>> {
        let mut buffer = BytesMut::with_capacity(8);

        buffer.put_u16(command.into());
        if let Some(args) = args {
            args.iter().for_each(|arg| {
                buffer.put_u16(*arg);
                buffer.put_u8(sensirion_i2c::crc8::calculate(&(*arg).to_be_bytes()[..]))
            });

            #[cfg(any(debug_assertions, feature = "log"))]
            log::trace!(
                "Writing to bus {{address: {:#x?}, command: {:?}[{}], args: [{}]}}",
                self.address,
                command,
                u16::from(command).to_be_bytes().iter().map(|b| alloc::format!("{:#x?}", b)).collect::<alloc::vec::Vec<_>>().join(", "),
                &buffer[2..].iter().map(|b| alloc::format!("{:#x?}", b)).collect::<alloc::vec::Vec<_>>().join(", "),
            );
        } else {
            #[cfg(any(debug_assertions, feature = "log"))]
            log::trace!(
                "Writing to bus {{address: {:#x?}, command: {:?}[{}]}}",
                self.address,
                command,
                u16::from(command).to_be_bytes().iter().map(|b| alloc::format!("{:#x?}", b)).collect::<alloc::vec::Vec<_>>().join(", "),
            );
        }

        self.i2c
            .write(self.address, &buffer[..])
            .map_err(sensirion_i2c::i2c::Error::<I2C>::I2cWrite)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::u16;

    use super::*;
    use crate::AddressPin;
    use embedded_hal_mock::eh1::{delay::NoopDelay, i2c::Transaction};
    use measurements::{Humidity, Temperature};

    fn create_i2c<
        T: FnOnce(Sht3x<&mut embedded_hal_mock::common::Generic<Transaction>, u8, NoopDelay>),
    >(
        expectations: &[Transaction],
        action: T,
    ) {
        let mut i2c_mock = embedded_hal_mock::eh1::i2c::Mock::new(expectations);
        let sht3x = create_device(&mut i2c_mock);
        action(sht3x);
        i2c_mock.done();
    }

    fn create_device(
        i2c: &mut embedded_hal_mock::common::Generic<Transaction>,
    ) -> Sht3x<&mut embedded_hal_mock::common::Generic<Transaction>, u8, NoopDelay> {
        Sht3x::new(i2c, AddressPin::default().into(), NoopDelay::default())
    }

    #[test]
    fn test_new() {
        create_i2c(&[], |sht3x| {
            log::info!("here {}", sht3x.address);
        });
    }

    #[test]
    fn test_command_break() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::Break).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, |mut sht3x| {
            assert_eq!((), sht3x.periodic_measurement_stop().unwrap())
        });
    }

    #[test]
    fn test_command_fetchdata() {
        let measurement = Measurement {
            temperature: Temperature::from_celsius(50.0),
            relative_humidity: Humidity::from_percent(50.0),
        };

        let expectations = [
            Transaction::write(
                AddressPin::default().into(),
                u16::from(Command::FetchData).to_be_bytes().to_vec(),
            ),
            Transaction::read(AddressPin::default().into(), measurement.clone().into()),
        ];

        create_i2c(&expectations, |mut sht3x| {
            let i2c_measurement = sht3x.periodic_measurement_fetch().unwrap();
            assert!(
                (i2c_measurement.relative_humidity.as_percent()
                    - measurement.relative_humidity.as_percent())
                .abs()
                    < 0.01
            );
            assert!(
                (i2c_measurement.temperature.as_celsius() - measurement.temperature.as_celsius())
                    .abs()
                    < 0.01
            );
        });
    }

    #[test]
    fn test_command_heater_disable() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::HeaterDisable).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, |mut sht3x| {
            assert_eq!((), sht3x.heater_disable().unwrap())
        });
    }

    #[test]
    fn test_command_heater_enable() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::HeaterEnable).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, |mut sht3x| {
            assert_eq!((), sht3x.heater_enable().unwrap())
        });
    }

    #[test]
    fn test_command_periodic_measurements_start() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::Periodic(Rate::R1, Repeatability::High))
                .to_be_bytes()
                .to_vec(),
        )];

        create_i2c(&expectations, |mut sht3x| {
            assert_eq!(
                (),
                sht3x
                    .periodic_measurements_start(Repeatability::High, Rate::R1)
                    .unwrap()
            );
        });
    }

    #[test]
    fn test_command_periodic_measurement_start_with_art() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::PeriodicWithART).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, |mut sht3x| {
            assert_eq!((), sht3x.periodic_measurement_start_with_art().unwrap());
        });
    }

    #[test]
    fn test_command_measure_singleshot() {
        let measurement = Measurement {
            temperature: Temperature::from_celsius(50.0),
            relative_humidity: Humidity::from_percent(50.0),
        };

        let expectations = [
            Transaction::write(
                AddressPin::default().into(),
                u16::from(Command::SingleShot(
                    crate::ClockStretching::Disabled,
                    Repeatability::High,
                ))
                .to_be_bytes()
                .to_vec(),
            ),
            Transaction::read(AddressPin::default().into(), measurement.clone().into()),
        ];

        create_i2c(&expectations, |mut sht3x| {
            let i2c_measurement = sht3x.measure_singleshot(Repeatability::High).unwrap();
            assert!(
                (i2c_measurement.relative_humidity.as_percent()
                    - measurement.relative_humidity.as_percent())
                .abs()
                    < 0.01
            );
            assert!(
                (i2c_measurement.temperature.as_celsius() - measurement.temperature.as_celsius())
                    .abs()
                    < 0.01
            );
        });
    }

    #[test]
    fn test_command_soft_reset() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::SoftReset).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, |mut sht3x| {
            assert_eq!((), sht3x.soft_reset().unwrap());
        });
    }

    #[test]
    fn test_command_status() {
        let status = Status::from_bits_retain(u16::MAX);

        let expectations = [
            Transaction::write(
                AddressPin::default().into(),
                u16::from(Command::Status).to_be_bytes().to_vec(),
            ),
            Transaction::read(AddressPin::default().into(), status.clone().into()),
        ];

        create_i2c(&expectations, |mut sht3x| {
            let i2c_status = sht3x.status().unwrap();
            assert_eq!(status.is_all(), i2c_status.is_all());
            assert!(status.heater_enabled());
            assert!(status.write_data_checksum_error());
            assert!(status.command_execution_failed());
            assert!(status.system_reset_detected());
            assert!(status.temperature_tracking_alert_occurred());
            assert!(status.relative_humidity_tracking_alert_occurred());
            assert!(status.alert_pending());
        });
    }

    #[test]
    fn test_command_status_clear() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::StatusClear).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, |mut sht3x| {
            assert_eq!((), sht3x.status_clear().unwrap());
        });
    }
}
