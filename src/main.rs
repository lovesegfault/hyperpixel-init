use anyhow::{Context, Result};
use byteorder::{BigEndian, ByteOrder};
use libc::{MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};
use log::{debug, info, warn};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
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
        let gpiomem = PathBuf::from("/dev/gpiomem-404");
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

    fn parse_address_cells<P: AsRef<std::path::Path>>(path: P) -> Result<u32> {
        let path = path.as_ref();
        if !path.exists() {
            // https://readthedocs.org/projects/devicetree-specification/downloads/pdf/stable/
            // c.f. p14
            // "missing, a client program should assume a default value of 2
            // for #address-cells"
            warn!("No #address-cells in {}", path.display());
            return Ok(2);
        }

        let mut address_cells_fd = File::open(path)
            .with_context(|| format!("Failed to open #address-cells {}", path.display()))?;

        let mut address_cells_buf = [0_u8; 4];
        address_cells_fd
            .read(&mut address_cells_buf)
            .with_context(|| format!("Failed to read #address-cells {}", path.display()))?;

        Ok(BigEndian::read_u32(&address_cells_buf))
    }

    fn parse_size_cells<P: AsRef<std::path::Path>>(path: P) -> Result<u32> {
        let path = path.as_ref();
        if !path.exists() {
            // https://readthedocs.org/projects/devicetree-specification/downloads/pdf/stable/
            // c.f. p14
            // "missing, a client program should assume a default value of 1
            // for #size-cells"
            warn!("No #size-cells in {}", path.display());
            return Ok(1);
        }

        let mut size_cells_fd = File::open(path)
            .with_context(|| format!("Failed to open #size-cells {}", path.display()))?;

        let mut size_cells_buf = [0_u8; 4];
        size_cells_fd
            .read(&mut size_cells_buf)
            .with_context(|| format!("Failed to read #size-cells {}", path.display()))?;

        Ok(BigEndian::read_u32(&size_cells_buf))
    }

    fn parse_ranges<P: AsRef<std::path::Path>>(
        path: P,
        child_size: u32,
        parent_size: u32,
        length_size: u32,
    ) -> Result<(u64, u64, u64)> {
        let path = path.as_ref();
        anyhow::ensure!(
            path.exists(),
            format!("Ranges {} does not exist", path.display())
        );

        let mut ranges_fd = File::open(path)
            .with_context(|| format!("Failed to open ranges {}", path.display()))?;

        let mut ranges: Vec<u8> = Vec::new();
        ranges_fd
            .read_to_end(&mut ranges)
            .with_context(|| format!("Failed to read ranges {}", path.display()))?;

        let gpio_range = ranges
            .chunks_exact(((child_size + parent_size + length_size) * 4) as usize)
            .map(|range| {
                let mut range = Vec::from(range);
                let child_addr = match child_size {
                    1 => BigEndian::read_u32(&range.drain(0..4).collect::<Vec<u8>>()) as u64,
                    2 => BigEndian::read_u64(&range.drain(0..8).collect::<Vec<u8>>()),
                    _ => anyhow::bail!(format!("Invalid child size of {}", child_size)),
                };

                let parent_addr = match parent_size {
                    1 => BigEndian::read_u32(&range.drain(0..4).collect::<Vec<u8>>()) as u64,
                    2 => BigEndian::read_u64(&range.drain(0..8).collect::<Vec<u8>>()),
                    _ => anyhow::bail!(format!("Invalid parent size of {}", parent_size)),
                };

                let length = match length_size {
                    1 => BigEndian::read_u32(&range.drain(0..4).collect::<Vec<u8>>()) as u64,
                    2 => BigEndian::read_u64(&range.drain(0..8).collect::<Vec<u8>>()),
                    _ => anyhow::bail!(format!("Invalid length size of {}", length_size)),
                };
                Ok((child_addr, parent_addr, length))
            })
            .filter_map(Result::ok)
            .find(|&(child_addr, _, length)| {
                (child_addr < 0x7e20_0000) && (child_addr + length > 0x7e20_0000)
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to find valid range"))?;

        Ok(gpio_range)
    }

    fn find_host_peripheral_address() -> Result<u64> {
        // 1. find & parse #address-cels, soc/#{address-cells,size-cells}
        let address_cells = Self::parse_address_cells("/proc/device-tree/#address-cells")?;
        let soc_address_cells = Self::parse_address_cells("/proc/device-tree/soc/#address-cells")?;
        let soc_size_cells = Self::parse_size_cells("/proc/device-tree/soc/#size-cells")?;
        debug!(
            "sizes {:#x} {:#x} {:#x}",
            soc_address_cells, address_cells, soc_size_cells
        );
        // 2. parse the (child_addr, parent_addr, length) triple from /proc/device-tree/soc/ranges
        // https://readthedocs.org/projects/devicetree-specification/downloads/pdf/stable/
        // p. 15
        let (child_addr, parent_addr, length) = Self::parse_ranges(
            "/proc/device-tree/soc/ranges",
            soc_address_cells,
            address_cells,
            soc_size_cells,
        )?;
        debug!("{:#x} {:#x} {:#x}", child_addr, parent_addr, length);

        let gpio_addr = (0x7e20_0000 - child_addr) + parent_addr;

        Ok(gpio_addr)
    }

    fn compute_gpio_mem() -> Result<*mut u32> {
        let mem = PathBuf::from("/dev/mem");
        anyhow::ensure!(mem.exists(), "Failed to find /dev/mem");

        let mem = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/mem")
            .with_context(|| "Failed to open /dev/mem")?;

        let bcm_phys_addr = Self::find_host_peripheral_address()
            .with_context(|| "Failed to find host's peripheral address")?;
        debug!("bcm_phys_addr: {:#x}", bcm_phys_addr);

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
            info!("Using /dev/gpiomem");
            debug!("/dev/gpiomem = {:#?}", addr);
            return Ok(Self(addr));
        } else {
            let addr = Self::compute_gpio_mem()?;
            info!("Using /dev/mem");
            debug!("/dev/mem = {:#?}", addr);
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
    env_logger::init();
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
