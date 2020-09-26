mod gpio;
mod hyperpixel;

use anyhow::{Context, Result};
use gpio::{Gpio, PinMode};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .with_context(|| "Unable to set global default subscriber")?;

    info!("HyperPixel 4 Initialization");

    let uid = unsafe { libc::getuid() };
    anyhow::ensure!(uid == 0, "Not running as root");

    let mut gpio = Gpio::new()?;

    info!("Setting Pin Modes");
    (0..10)
        .chain(12..18)
        .chain(20..26)
        .map(|pin| gpio.set_pin_mode(pin, PinMode::Alt2))
        .collect::<Result<()>>()?;

    info!("Configuring Display");
    hyperpixel::hyperpixel_configure(&mut gpio)?;

    Ok(())
}
