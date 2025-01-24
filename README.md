![Crates.io Version](https://img.shields.io/crates/v/am2301)

# am2301-rs

A crate to interface with an AM2301 humidity and temperature sensor using a
Raspberry Pi Pico 1 (RP2040) microcontroller and the embassy framework.

Because the sensor uses a custom one-wire protocol with tight timings the
measure is blocking and and expected to take around 5ms (the sensor cannot be
pulled sooner than every 2s anyway, as per its datasheet).

A basic example can be found in the `examples` directory.
