#![no_std]

mod measure;

use defmt::Format;
use embassy_rp::gpio::Flex;
use measure::ReadBitsError;

enum ProcessResponseError {
    InvalidChecksumError,
    InvalidNumberOfBits,
}

impl From<core::array::TryFromSliceError> for ProcessResponseError {
    fn from(_: core::array::TryFromSliceError) -> Self {
        ProcessResponseError::InvalidNumberOfBits
    }
}

fn process_response(bits: [u8; 40]) -> Result<(f64, f64), ProcessResponseError> {
    let byte1 = <[u8; 8]>::try_from(&bits[0..8])?;
    let byte2 = <[u8; 8]>::try_from(&bits[8..16])?;
    let byte3 = <[u8; 8]>::try_from(&bits[16..24])?;
    let byte4 = <[u8; 8]>::try_from(&bits[24..32])?;
    let byte5 = <[u8; 8]>::try_from(&bits[32..40])?;

    let checksum_left = [byte1, byte2, byte3, byte4]
        .iter()
        .map(convert_byte_to_u8)
        .fold(0u8, |acc, x| acc.wrapping_add(x));
    let checksum_right = convert_byte_to_u8(&byte5);

    if checksum_left != checksum_right {
        return Err(ProcessResponseError::InvalidChecksumError);
    }

    let mut humidity_bits = [0u8; 16];
    humidity_bits[0..8].copy_from_slice(&byte1);
    humidity_bits[8..16].copy_from_slice(&byte2);

    let mut humidity = 0;
    for (idx, &bit) in humidity_bits.iter().rev().enumerate() {
        humidity += bit as u16 * 2u16.pow(idx as u32);
    }

    let temperature_sign = if byte3[0] == 1 { -1 } else { 1 };
    let mut temperature_bits = [0u8; 15];
    temperature_bits[0..7].copy_from_slice(&byte3[1..8]);
    temperature_bits[7..15].copy_from_slice(&byte4);
    let mut temperature = 0;
    for (idx, &bit) in temperature_bits.iter().rev().enumerate() {
        temperature += bit as i16 * 2i16.pow(idx as u32);
    }
    temperature *= temperature_sign;

    Ok((humidity as f64 * 0.1, temperature as f64 * 0.1))
}

fn convert_byte_to_u8(byte: &[u8; 8]) -> u8 {
    let mut value = 0;
    for (idx, &bit) in byte.iter().rev().enumerate() {
        value += bit * 2u8.pow(idx as u32);
    }
    value
}

#[derive(Format)]
/// Possible ways a measure can fail.
pub enum MeasureError {
    /// A timeout occured during the measure.
    MeasureTimeoutError,
    /// The checksum of the measure does not match its content.
    ChecksumError,
    /// Invalid measure.
    MeasureError,
}

impl From<ProcessResponseError> for MeasureError {
    fn from(value: ProcessResponseError) -> Self {
        match value {
            ProcessResponseError::InvalidChecksumError => Self::ChecksumError,
            _ => Self::MeasureError,
        }
    }
}

impl From<ReadBitsError> for MeasureError {
    fn from(_: ReadBitsError) -> Self {
        MeasureError::MeasureTimeoutError
    }
}

#[deprecated(
    since = "0.2.0",
    note = "Has not timeout, could block forever. Use measure_once_timeout instead."
)]
pub async fn measure_once(pin: &mut Flex<'_>) -> Result<(f64, f64), MeasureError> {
    let bits = measure::read_bits(pin)?;
    let (humidity, temperature) = process_response(bits)?;
    Ok((humidity, temperature))
}

pub struct Measure {
    /// Humidity in % between \[0, 100\].
    pub humidity: f64,
    /// Temperature in degree Celsius.
    pub temperature: f64,
}

/// Retrieve a single measure from the sensor connected in pin.
/// Will timeout if no matching sensor is connected to the pin.
pub async fn measure_once_timeout(pin: &mut Flex<'_>) -> Result<Measure, MeasureError> {
    let bits = measure::read_bits_timeout(pin)?;
    process_response(bits)
        .map(|(humidity, temperature)| Measure {
            humidity,
            temperature,
        })
        .map_err(MeasureError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_conversion() {
        // 00000010 10010010 00000001 00001101 10100010 example from datasheet
        #[rustfmt::skip]
        let bits = [
            0, 0, 0, 0, 0, 0, 1, 0,
            1, 0, 0, 1, 0, 0, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0, 1, 1, 0, 1,
            1, 0, 1, 0, 0, 0, 1, 0,
        ];
        match process_response(bits) {
            Ok((humidity, temperature)) => {
                let expected_humidity = 65.8;
                assert!((humidity - expected_humidity).abs() < 0.01);
                let expected_temperature = 26.9;
                assert!((temperature - expected_temperature).abs() < 0.01);
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn fail_when_checksum_does_not_match() {
        // 00000010 10010010 00000001 00001101 10110010 example from datasheet
        #[rustfmt::skip]
        let bits = [
            0, 0, 0, 0, 0, 0, 1, 0,
            1, 0, 0, 1, 0, 0, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0, 1, 1, 0, 1,
            1, 0, 1, 1, 0, 0, 1, 0,
        ];
        let res = process_response(bits);

        assert!(res.is_err());
    }

    #[test]
    fn fail_ok_when_checksum_overflow() {
        #[rustfmt::skip]
        let bits = [
            0, 0, 0, 0, 0, 0, 1, 0,
            1, 0, 0, 1, 0, 0, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0, 1, 1, 0, 1,
            1, 0, 1, 0, 0, 0, 1, 0,
        ];
        let res = process_response(bits);

        assert!(res.is_ok());
    }

    #[test]
    fn test_with_negative_temperature() {
        #[rustfmt::skip]
        let bits = [
            0, 0, 0, 0, 0, 0, 1, 0,
            0, 0, 0, 1, 0, 0, 1, 0,
            1, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0, 1, 1, 0, 1,
            1, 0, 1, 0, 0, 0, 1, 0,
        ];

        match process_response(bits) {
            Ok((_, temperature)) => {
                let expected_temperature = -26.9;
                assert!((temperature - expected_temperature).abs() < 0.01);
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn u8_addition_overflow() {
        let num1 = 250u8;
        let num2 = 100u8;

        assert_eq!(num1.wrapping_add(num2), 94);
    }
}
