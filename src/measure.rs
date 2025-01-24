use embassy_rp::gpio::Flex;
use embassy_time::{block_for, Duration, Instant};

const TIMEOUT_DURATION: Duration = Duration::from_micros(100);

fn trigger_measure(pin: &mut Flex<'_>) {
    pin.set_high();
    pin.set_as_output();

    // Set to low for 1ms
    pin.set_low();
    block_for(Duration::from_millis(1));
    pin.set_high();

    pin.set_as_input();
}

fn wait_for_falling_edge(pin: &mut Flex<'_>) -> u8 {
    let start = Instant::now();
    let mut pin_is_low = pin.is_low();
    while !pin_is_low {
        pin_is_low = pin.is_low();
        block_for(Duration::from_micros(1));
    }
    start.elapsed().as_micros() as u8
}

fn wait_for_rising_edge(pin: &mut Flex<'_>) -> u8 {
    let start = Instant::now();
    let mut pin_is_high = pin.is_high();
    while !pin_is_high {
        pin_is_high = pin.is_high();
    }
    start.elapsed().as_micros() as u8
}

fn wait_for_falling_edge_timeout(pin: &mut Flex<'_>) -> Option<u8> {
    let start = Instant::now();
    while pin.is_high() {
        if start.elapsed() > TIMEOUT_DURATION {
            return None;
        }
        block_for(Duration::from_micros(1));
    }
    Some(start.elapsed().as_micros() as u8)
}

fn wait_for_rising_edge_timeout(pin: &mut Flex<'_>) -> Option<u8> {
    let start = Instant::now();
    while pin.is_low() {
        if start.elapsed() > TIMEOUT_DURATION {
            return None;
        }
        // Not blocking here, as it tends to create a lot of timeout
        // block_for(Duration::from_micros(1));
    }
    Some(start.elapsed().as_micros() as u8)
}

fn skip_start_of_measure(pin: &mut Flex<'_>) {
    // Measure starts with a falling edge, a rising edge, and a final falling edge.
    wait_for_falling_edge(pin);
    wait_for_rising_edge(pin);
    wait_for_falling_edge(pin);
}

pub enum ReadBitsError {
    TimeoutErr,
}

pub fn read_bits(pin: &mut Flex<'_>) -> Result<[u8; 40], ReadBitsError> {
    let mut measures = [0u8; 40];
    trigger_measure(pin);
    pin.set_as_input();

    skip_start_of_measure(pin);

    for measure in measures.iter_mut() {
        wait_for_rising_edge(pin);
        let delay = wait_for_falling_edge(pin);
        *measure = match delay {
            d if d > 50 => 1,
            _ => 0,
        };
    }

    Ok(measures)
}

pub fn read_bits_timeout(pin: &mut Flex<'_>) -> Result<[u8; 40], ReadBitsError> {
    let mut measures = [0u8; 40];

    trigger_measure(pin);
    pin.set_as_input();

    skip_start_of_measure(pin);

    for measure in measures.iter_mut() {
        wait_for_rising_edge_timeout(pin).ok_or(ReadBitsError::TimeoutErr)?;
        let delay = wait_for_falling_edge_timeout(pin).ok_or(ReadBitsError::TimeoutErr)?;
        *measure = match delay {
            d if d > 50 => 1,
            _ => 0,
        };
    }

    Ok(measures)
}
