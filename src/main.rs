mod gpio;
mod hyperpixel;

use anyhow::Result;
use gpio::{Gpio, PinMode};
use log::info;

fn main() -> Result<()> {
    env_logger::init();
    info!("HyperPixel 4 Initialization");

    let uid = unsafe { libc::getuid() };
    anyhow::ensure!(uid == 0, "Not running as root");

    let mut gpio = Gpio::new()?;

    info!("Setting Pin Modes");
    (0..10)
        .into_iter()
        .chain(12..18)
        .chain(20..26)
        .map(|pin| gpio.set_pin_mode(pin, PinMode::Alt2))
        .collect::<Result<()>>()?;

    info!("Configuring Display");
    hyperpixel::hyperpixel_configure(&mut gpio)?;

    Ok(())
}
