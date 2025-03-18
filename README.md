# sensirion-sht3x

This is a platform agnostic driver for Sensirion SHT3x series of temperature and humidity sensors using traits from [`embedded-hal`](https://docs.rs/embedded-hal/latest/embedded_hal/) and [`embedded-hal-async`](https://docs.rs/embedded-hal-async/latest/embedded_hal_async/) to allow generalized use on any [`no_std`](https://docs.rust-embedded.org/book/intro/no-std.html) platform. It provides both a blocking interface by default and
an asynchronous interface by enabling the `embedded-hal-async` feature.

## Device

The digital SHT3x humidity sensor series takes sensor technology to a new level. As the successor of the SHT2x series it sets the industry standard in humidity sensing. The SHT3x humidity sensor series consists of a low-cost version with the SHT30 humidity sensor, a standard version with the SHT31 humidity sensor, and a high-end version with the SHT35 humidity sensor. The SHT3x humidity sensor series combines multiple functions and various interfaces (I<sup>2</sup>C, analog voltage output) with an applications-friendly, very wide operating voltage range (2.15 to 5.5 V). 

| Model  | Humidity Accuracy | Temperature Accuracy | Use Case |
|--------|------------------|---------------------|----------|
| SHT30  | ±3% RH          | ±0.3°C              | Cost-sensitive applications |
| SHT31  | ±2% RH          | ±0.3°C              | Home automation, HVAC |
| SHT35  | ±1.5% RH        | ±0.2°C              | Industrial, medical |

> [!IMPORTANT]  
> This library operates under the assumption that the sensor is supplied with a voltage of ≥2.4 V for accurate measurement timing.

> [!NOTE]  
> As this library uses hal implementations of generic traits to interface with hardware some functionality could differ from platform to platform. Specifically
> users of this library should be sure their platform supports any feature they're trying to use. As an example ESP32 I<sup>2</sup>C controllers don't
> natively support clock stretching calling `measure_singleshot_with_clock_stretching` would have undefined behavior.

- [Datasheet](https://sensirion.com/media/documents/213E6A3B/63A5A569/Datasheet_SHT3x_DIS.pdf)

## Features

| Feature                               | Blocking  | Async |
| ------------------------------------- | --------- | ----- |
| Fetch status                          | ✅        | ✅    |
| Clear status                          | ✅        | ✅    |
| Enable heater                         | ✅        | ✅    |
| Disable heater                        | ✅        | ✅    |
| Single measurement                    | ✅        | ✅    |
| Single measurement w/clock stretching | ✅        | ✅    |
| Periodic measurement                  | ✅        | ✅    |


## Usage

```rust,no_run
use sensirion_sht3x::{AddressPin, Repeatability, blocking::Sht3x};
use embedded_hal_mock::eh1::{delay::NoopDelay, i2c};

fn main() {
    // Create the I2C device from the chosen embedded-hal implementation,
    // in this case embedded_hal_mock
    let mut i2c = i2c::Mock::new(&[]);

    // Create the sensor
    let mut sensor = Sht3x::new(i2c, AddressPin::High.into(), NoopDelay::default());

    // Perform measurement
    let measurement = sensor.measure_singleshot(Repeatability::High).expect("Unable to get measurement");

    println!(
        "Temperature: {:.2} °C, Relative humidity: {:.2} %",
        measurement.temperature,
        measurement.relative_humidity
    );
}
```
