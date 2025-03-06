#[derive(Debug)]
/// Represents all potential errors that can be produced by the Sht3x struct.
pub enum Error<E>
where
    E: embedded_hal::i2c::Error,
{
    /// Error occurring on the I²C bus.
    I2c(E),

    /// Failed CRC checksum validation.
    Crc,
}

impl<I: embedded_hal::i2c::ErrorType> From<sensirion_i2c::i2c::Error<I>> for Error<I::Error> {
    fn from(value: sensirion_i2c::i2c::Error<I>) -> Self {
        match value {
            sensirion_i2c::i2c::Error::I2cWrite(e) => Error::I2c(e),
            sensirion_i2c::i2c::Error::I2cRead(e) => Error::I2c(e),
            sensirion_i2c::i2c::Error::Crc => Error::Crc,
            sensirion_i2c::i2c::Error::InvalidCommand => {
                panic!(
                    "The value sent as an I²C command couldn't be converted \
                     into a valid command."
                )
            }
        }
    }
}
