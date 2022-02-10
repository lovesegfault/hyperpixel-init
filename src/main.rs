mod gpio;
mod hyperpixel;

use anyhow::{Context, Result};
use flexi_logger::Logger;
use gpio::{Gpio, PinMode};
use log::info;

fn main() -> Result<()> {
    Logger::try_with_env_or_str("info")
        .with_context(|| "Failed to parse log level")?
        .format(flexi_logger::colored_detailed_format)
        .set_palette("196;228;120;45;176".to_string())
        .start()
        .with_context(|| "Failed to initialize logger")?;

    info!("HyperPixel 4 Initialization");

    let uid = unsafe { libc::getuid() };
    anyhow::ensure!(uid == 0, "Not running as root");

    let mut gpio = Gpio::new()?;

    info!("Setting Pin Modes");
    (0..10)
        .chain(12..18)
        .chain(20..26)
        .try_for_each(|pin| gpio.set_pin_mode(pin, PinMode::Alt2))?;

    info!("Configuring Display");
    hyperpixel::hyperpixel_configure(&mut gpio)?;

    info!("Done!");
    Ok(())
}
