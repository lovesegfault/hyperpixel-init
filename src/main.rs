use anyhow::{Context, Result};
use byteorder::{BigEndian, ByteOrder};
use libc::{MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};
use log::{debug, info, warn};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

const CLK: u32 = 27;
const MOSI: u32 = 26;
const CS: u32 = 18;
const DELAY: u64 = 100;
const COMMANDS: &[i16] = &[
    0x0ff, 0x1ff, 0x198, 0x106, 0x104, 0x101, 0x008, 0x110, 0x021, 0x109, 0x030, 0x102, 0x031,
    0x100, 0x040, 0x110, 0x041, 0x155, 0x042, 0x102, 0x043, 0x109, 0x044, 0x107, 0x050, 0x178,
    0x051, 0x178, 0x052, 0x100, 0x053, 0x16d, 0x060, 0x107, 0x061, 0x100, 0x062, 0x108, 0x063,
    0x100, 0x0a0, 0x100, 0x0a1, 0x107, 0x0a2, 0x10c, 0x0a3, 0x10b, 0x0a4, 0x103, 0x0a5, 0x107,
    0x0a6, 0x106, 0x0a7, 0x104, 0x0a8, 0x108, 0x0a9, 0x10c, 0x0aa, 0x113, 0x0ab, 0x106, 0x0ac,
    0x10d, 0x0ad, 0x119, 0x0ae, 0x110, 0x0af, 0x100, 0x0c0, 0x100, 0x0c1, 0x107, 0x0c2, 0x10c,
    0x0c3, 0x10b, 0x0c4, 0x103, 0x0c5, 0x107, 0x0c6, 0x107, 0x0c7, 0x104, 0x0c8, 0x108, 0x0c9,
    0x10c, 0x0ca, 0x113, 0x0cb, 0x106, 0x0cc, 0x10d, 0x0cd, 0x118, 0x0ce, 0x110, 0x0cf, 0x100,
    0x0ff, 0x1ff, 0x198, 0x106, 0x104, 0x106, 0x000, 0x120, 0x001, 0x10a, 0x002, 0x100, 0x003,
    0x100, 0x004, 0x101, 0x005, 0x101, 0x006, 0x198, 0x007, 0x106, 0x008, 0x101, 0x009, 0x180,
    0x00a, 0x100, 0x00b, 0x100, 0x00c, 0x101, 0x00d, 0x101, 0x00e, 0x100, 0x00f, 0x100, 0x010,
    0x1f0, 0x011, 0x1f4, 0x012, 0x101, 0x013, 0x100, 0x014, 0x100, 0x015, 0x1c0, 0x016, 0x108,
    0x017, 0x100, 0x018, 0x100, 0x019, 0x100, 0x01a, 0x100, 0x01b, 0x100, 0x01c, 0x100, 0x01d,
    0x100, 0x020, 0x101, 0x021, 0x123, 0x022, 0x145, 0x023, 0x167, 0x024, 0x101, 0x025, 0x123,
    0x026, 0x145, 0x027, 0x167, 0x030, 0x111, 0x031, 0x111, 0x032, 0x100, 0x033, 0x1ee, 0x034,
    0x1ff, 0x035, 0x1bb, 0x036, 0x1aa, 0x037, 0x1dd, 0x038, 0x1cc, 0x039, 0x166, 0x03a, 0x177,
    0x03b, 0x122, 0x03c, 0x122, 0x03d, 0x122, 0x03e, 0x122, 0x03f, 0x122, 0x040, 0x122, 0x052,
    0x110, 0x053, 0x110, 0x0ff, 0x1ff, 0x198, 0x106, 0x104, 0x107, 0x018, 0x11d, 0x017, 0x122,
    0x002, 0x177, 0x026, 0x1b2, 0x0e1, 0x179, 0x0ff, 0x1ff, 0x198, 0x106, 0x104, 0x100, 0x03a,
    0x160, 0x035, 0x100, 0x011, 0x100, -1, 0x029, 0x100, -1,
];

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

    pub fn set_level(&mut self, pin: u32, level: bool) -> Result<()> {
        let register = (pin / 32) as isize;
        let pin_shift = (pin % 32) * 1;

        let level = match level {
            true => unsafe { self.0.offset(0x1c / 0x4) },
            false => unsafe { self.0.offset(0x28 / 0x4) },
        };

        let pin_level = unsafe { level.offset(register) };
        unsafe { std::ptr::write_volatile(pin_level, 1 << pin_shift) };
        Ok(())
    }
    fn write_9bit(&mut self, command: u16) -> Result<()> {
        self.set_level(CS, false)?;
        self.send_bits(command, 9)?;
        self.set_level(CS, true)?;
        Ok(())
    }
    fn send_bits(&mut self, data: u16, count: u16) -> Result<()> {
        let mut mask = 1 << (count - 1);
        for _ in 0..count {
            self.set_level(MOSI, (data & mask) > 0)?;
            mask >>= 1;
            self.set_level(CLK, false)?;
            sleep(Duration::from_micros(DELAY));
            self.set_level(CLK, true)?;
            sleep(Duration::from_micros(DELAY));
        }
        self.set_level(MOSI, false)?;
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

    gpio.set_level(CLK, false)?;
    gpio.set_level(MOSI, false)?;
    gpio.set_level(CS, true)?;

    gpio.set_pin(CLK, PinMode::Out)?;
    gpio.set_pin(MOSI, PinMode::Out)?;
    gpio.set_pin(CS, PinMode::Out)?;

    const WAIT: u64 = 120_000; // micros
    COMMANDS.iter().for_each(|c| match c {
        -1 => std::thread::sleep(std::time::Duration::from_micros(WAIT)),
        c => gpio.write_9bit(*c as u16).unwrap(), // fixme
    });

    gpio.set_pin(CLK, PinMode::In)?;
    gpio.set_pin(MOSI, PinMode::In)?;
    gpio.set_pin(CS, PinMode::In)?;
    Ok(())
}
