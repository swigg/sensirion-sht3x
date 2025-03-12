use crate::{command::Command, Measurement, Rate, Repeatability, Status};
use bytes::{Buf, BytesMut};
use embedded_hal_async::i2c::{Operation, SevenBitAddress};
use sensirion_core::{asynchronous::SensirionI2c, Error};

/// Represents an async driver for the SHT3x device.
#[derive(Default)]
pub struct Sht3x<I2C, A, D> {
    i2c: I2C,
    address: A,
    delay: D,
}

impl<I2C, D> SensirionI2c<I2C, SevenBitAddress, D> for Sht3x<I2C, SevenBitAddress, D>
where
    I2C: embedded_hal_async::i2c::I2c,
    D: embedded_hal_async::delay::DelayNs,
{
    fn i2c(&mut self) -> &mut I2C {
        &mut self.i2c
    }

    fn address(&mut self) -> SevenBitAddress {
        self.address
    }

    fn delay(&mut self) -> &mut D {
        &mut self.delay
    }
}

impl<I2C, D> Sht3x<I2C, SevenBitAddress, D>
where
    I2C: embedded_hal_async::i2c::I2c,
    D: embedded_hal_async::delay::DelayNs,
{
    /// Instantiate a new async SHT3x device driver.
    pub fn new(i2c: I2C, address: SevenBitAddress, delay: D) -> Self {
        Self {
            address,
            i2c,
            delay,
        }
    }

    /// Perform a single-shot measurement with the given repeatability.
    pub async fn measure_singleshot(
        &mut self,
        repeatability: Repeatability,
    ) -> Result<Measurement, Error<I2C::Error>> {
        let mut measurement = BytesMut::with_capacity(6);
        measurement.resize(6, 0);

        self.read_command(
            Command::SingleShot(crate::ClockStretching::Disabled, repeatability),
            &mut measurement,
        )
        .await
        .map(Measurement::from)
    }

    /// Perform a single-shot measurement with clock stretching.
    pub async fn measure_singleshot_with_clock_stretching(
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
            .await
            .map(|_| Measurement::from(measurement))
            .map_err(Error::I2c)
    }

    /// Begin publishing periodic measurements.
    pub async fn periodic_measurement_start(
        &mut self,
        repeatability: Repeatability,
        rate: Rate,
    ) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::Periodic(rate, repeatability))
            .await
    }

    /// Fetch the latest periodic measurement.
    pub async fn periodic_measurement_fetch(&mut self) -> Result<Measurement, Error<I2C::Error>> {
        let mut measurement = BytesMut::with_capacity(6);
        measurement.resize(6, 0);

        self.read_command(Command::FetchData, &mut measurement)
            .await
            .map(Measurement::from)
    }

    /// Start periodic measurements using Accelerated Response Time (ART).
    pub async fn periodic_measurement_start_with_art(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::PeriodicWithART).await
    }

    /// Stop publishing periodic measurements.
    pub async fn periodic_measurement_stop(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::Break).await
    }

    /// Issue a soft reset to the device.
    pub async fn soft_reset(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::SoftReset).await
    }

    /// Enable the onboard heater.
    pub async fn heater_enable(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::HeaterEnable).await
    }

    /// Disable the onboard heater.
    pub async fn heater_disable(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::HeaterDisable).await
    }

    /// Read the contents of the status register.
    pub async fn status_fetch(&mut self) -> Result<Status, Error<I2C::Error>> {
        let mut status = BytesMut::with_capacity(3);
        status.resize(3, 0);

        self.read_command(Command::StatusFetch, &mut status)
            .await
            .map(|status| Status::from_bits_retain(status.get(0..2).unwrap().get_u16()))
    }

    /// Clear the status register.
    pub async fn status_clear(&mut self) -> Result<(), Error<I2C::Error>> {
        self.write_command(Command::StatusClear).await
    }
}

#[cfg(test)]
mod tests {
    use core::ops::AsyncFnOnce;

    use super::*;
    use crate::AddressPin;
    use embedded_hal_mock::eh1::{delay::NoopDelay, i2c::Transaction};
    use measurements::{Humidity, Temperature};

    fn create_i2c<F>(expectations: &[Transaction], action: F)
    where
        F: AsyncFnOnce(Sht3x<&mut embedded_hal_mock::common::Generic<Transaction>, u8, NoopDelay>),
    {
        let mut i2c_mock = embedded_hal_mock::eh1::i2c::Mock::new(expectations);
        let device = create_device(&mut i2c_mock);
        futures::executor::block_on(action(device));
        i2c_mock.done();
    }

    fn create_device(
        i2c: &mut embedded_hal_mock::common::Generic<Transaction>,
    ) -> Sht3x<&mut embedded_hal_mock::common::Generic<Transaction>, u8, NoopDelay> {
        Sht3x::new(i2c, AddressPin::default().into(), NoopDelay::default())
    }

    #[test]
    fn test_new() {
        create_i2c(&[], async |device| {
            #[cfg(feature = "log")]
            log::info!("Address {}", device.address);
        });
    }

    #[test]
    fn test_command_break() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::Break).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, async |mut device| {
            assert_eq!((), device.periodic_measurement_stop().await.unwrap());
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

        create_i2c(&expectations, async |mut device| {
            let i2c_measurement = device.periodic_measurement_fetch().await.unwrap();
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

        create_i2c(&expectations, async |mut device| {
            assert_eq!((), device.heater_disable().await.unwrap());
        });
    }

    #[test]
    fn test_command_heater_enable() {
        let expectations = [Transaction::write(
            AddressPin::default().into(),
            u16::from(Command::HeaterEnable).to_be_bytes().to_vec(),
        )];

        create_i2c(&expectations, async |mut device| {
            assert_eq!((), device.heater_enable().await.unwrap());
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

        create_i2c(&expectations, async |mut device| {
            assert_eq!(
                (),
                device
                    .periodic_measurement_start(Repeatability::High, Rate::R1)
                    .await
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

        create_i2c(&expectations, async |mut device| {
            assert_eq!(
                (),
                device.periodic_measurement_start_with_art().await.unwrap()
            );
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

        create_i2c(&expectations, async |mut device| {
            let i2c_measurement = device
                .measure_singleshot(Repeatability::High)
                .await
                .unwrap();
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

        create_i2c(&expectations, async |mut device| {
            assert_eq!((), device.soft_reset().await.unwrap());
        });
    }

    #[test]
    fn test_command_status() {
        let status = Status::from_bits_retain(u16::MAX);

        let expectations = [
            Transaction::write(
                AddressPin::default().into(),
                u16::from(Command::StatusFetch).to_be_bytes().to_vec(),
            ),
            Transaction::read(AddressPin::default().into(), status.clone().into()),
        ];

        create_i2c(&expectations, async |mut device| {
            let i2c_status = device.status_fetch().await.unwrap();
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

        create_i2c(&expectations, async |mut device| {
            assert_eq!((), device.status_clear().await.unwrap());
        });
    }
}
