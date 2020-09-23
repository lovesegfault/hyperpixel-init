use anyhow::{Context, Result};
use libc::{MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};
use log::{debug, info};
use std::fs::OpenOptions;
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;

pub enum PinMode {
    In = 0,
    Out = 1,
    Alt5 = 2,
    Alt4 = 3,
    Alt0 = 4,
    Alt1 = 5,
    Alt2 = 6,
    Alt3 = 7,
}

pub struct Gpio(*mut u32);

impl Gpio {
    fn find_gpio_mem() -> Result<*mut u32> {
        let gpiomem = PathBuf::from("/dev/gpiomem");
        anyhow::ensure!(gpiomem.exists(), "Failed to find /dev/gpiomem");

        let gpiomem = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/gpiomem")
            .with_context(|| "Failed to open /dev/gpiomem")?;

        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                0x1000,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                gpiomem.into_raw_fd(),
                0x0,
            )
        };
        anyhow::ensure!(map != MAP_FAILED, "Failed to mmap /dev/gpiomem");

        return Ok(map as *mut u32);
    }

    fn compute_gpio_mem() -> Result<*mut u32> {
        let mem = PathBuf::from("/dev/mem");
        anyhow::ensure!(mem.exists(), "Failed to find /dev/mem");

        let mem = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/mem")
            .with_context(|| "Failed to open /dev/mem")?;

        let bcm_phys_addr = unsafe { bcm_host_sys::bcm_host_get_peripheral_address() };
        debug!("bcm_phys_addr: {}", bcm_phys_addr);

        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                0x1000,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                mem.into_raw_fd(),
                (bcm_phys_addr + 0x20000) as i64,
            )
        };
        anyhow::ensure!(map != MAP_FAILED, "Failed to mmap /dev/mem");

        return Ok(map as *mut u32);
    }

    pub fn new() -> Result<Self> {
        if let Ok(addr) = Self::find_gpio_mem() {
            return Ok(Self(addr));
        } else {
            let addr = Self::compute_gpio_mem()?;
            Ok(Self(addr))
        }
    }

    pub fn set_pin(&mut self, pin: u32, mode: PinMode) -> Result<()> {
        anyhow::ensure!(pin <= 27, "Attempt to set mode of invalid pin");
        // 7 FSEL registers control all 64 GPIO pins. pin / 10 gives the FSEL register for the pin
        // we want.
        let register: u32 = pin / 10;
        // Each register bank has 4 bytes, 32 bits. Each pin function selection is 3 bits, so that
        // each bank holds the settings for 10 pins. bits 30, 31 remain unused.
        // https://www.raspberrypi.org/documentation/hardware/raspberrypi/bcm2835/BCM2835-ARM-Peripherals.pdf
        // p. 92
        let pin_shift: u32 = (pin % 10) * 3;
        // offset our base address by our register, which will give us a ptr to one of the 7 FSEL
        // banks
        let fsel_addr = unsafe { self.0.offset(register as isize) };
        // read whole register bank
        let current_fsel = unsafe { std::ptr::read_volatile(fsel_addr) };
        // write our mode to the three _bits_ within the 32 _bits_ of the register value we read
        // mask to clear the 3 bits for our pin's mode
        let mask = !(0b111 << pin_shift);
        let clean_fsel = current_fsel & mask;
        // write new value
        let new_fsel = clean_fsel | ((mode as u32) << pin_shift);
        // write new register value to the map
        unsafe { std::ptr::write_volatile(fsel_addr, new_fsel) };
        Ok(())
    }
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    info!("HyperPixel 4 Initialization");

    let uid = unsafe { libc::getuid() };
    anyhow::ensure!(uid == 0, "Not running as root");

    let mut gpio = Gpio::new()?;

    (0..10)
        .into_iter()
        .chain(12..18)
        .chain(20..26)
        .map(|pin| gpio.set_pin(pin, PinMode::Alt2))
        .collect::<Result<()>>()?;

    Ok(())
}
