#![no_std]
#![no_main]

use defmt::*;

use am2301::measure_once;

use embassy_rp::gpio::Flex;
use embassy_rp::peripherals::PIN_21;
use embassy_time::{Instant, Timer};
use embassy_executor::Spawner;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
pub async fn measure_task(pin: PIN_21) -> ! {
    let mut pin = Flex::new(pin);

    // Wait for sensor to initialized
    Timer::after_secs(2).await;

    loop {
        let start = Instant::now();
        match measure_once(&mut pin).await {
            Ok((humidity, temperature)) => {
                info!("Temperature = {} and humidity = {}", temperature, humidity);
            }
            Err(err) => {
                warn!("Error while measure temperature and humidity: {:?}", err)
            }
        }
        let delay = 5 - start.elapsed().as_secs();
        info!("Sleeping for {}s", delay);
        Timer::after_secs(delay).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    unwrap!(spawner.spawn(measure_task(p.PIN_21)));
}