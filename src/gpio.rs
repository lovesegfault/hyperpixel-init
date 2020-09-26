use anyhow::{Context, Result};
use byteorder::{BigEndian, ByteOrder};
use libc::{MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};
use log::{debug, info, warn};
use std::{
    fs::{File, OpenOptions},
    io::prelude::*,
    os::unix::io::IntoRawFd,
    path::{Path, PathBuf},
};

#[allow(dead_code)] // We want to document the other modes even if we don't use them
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
    pub fn new() -> Result<Self> {
        let addr = match map_gpio_mem() {
            Ok(addr) => {
                info!("Using /dev/gpiomem");
                debug!("/dev/gpiomem = {:#?}", addr);
                addr
            }
            Err(e) => {
                warn!("Failed to find /dev/gpiomem: {}", e);
                debug!("Attempting to compute gpio mem offset");
                let addr = find_gpio_mem()?;
                info!("Using /dev/mem");
                debug!("/dev/mem = {:#?}", addr);
                addr
            }
        };
        Ok(Self(addr))
    }

    pub fn set_pin_mode(&mut self, pin: u32, mode: PinMode) -> Result<()> {
        anyhow::ensure!(pin <= 27, "Attempt to set mode of invalid pin");
        // 7 FSEL registers control all 64 GPIO pins. pin / 10 gives the FSEL register for the pin
        // we want.
        let register: u32 = pin / 10;
        debug!("pin: {}, register: {}", pin, register);
        // Each register bank has 4 bytes, 32 bits. Each pin function selection is 3 bits, so that
        // each bank holds the settings for 10 pins. bits 30, 31 remain unused.
        // https://www.raspberrypi.org/documentation/hardware/raspberrypi/bcm2835/BCM2835-ARM-Peripherals.pdf
        // p. 92
        let pin_shift: u32 = (pin % 10) * 3;
        debug!("pin: {}, shift: {}", pin, pin_shift);
        // offset our base address by our register, which will give us a ptr to one of the 7 FSEL
        // banks
        let function_select_addr = unsafe { self.0.offset(register as isize) };
        debug!(
            "pin: {}, function_select_addr: {:#p}",
            pin, function_select_addr
        );
        // read whole register bank
        let current_function_select = unsafe { std::ptr::read_volatile(function_select_addr) };
        debug!(
            "pin: {}, current_function_select: {}",
            pin, current_function_select
        );
        // write our mode to the three _bits_ within the 32 _bits_ of the register value we read
        // mask to clear the 3 bits for our pin's mode
        let mask = !(0b111 << pin_shift);
        let clean_fsel = current_function_select & mask;
        // write new value
        let new_fsel = clean_fsel | ((mode as u32) << pin_shift);
        // write new register value to the map
        unsafe { std::ptr::write_volatile(function_select_addr, new_fsel) };
        Ok(())
    }

    pub fn set_pin_level(&mut self, pin: u32, level: bool) -> Result<()> {
        let register = (pin / 32) as isize;
        let pin_shift = pin % 32;

        let level = match level {
            true => unsafe { self.0.offset(0x1c / 0x4) },
            false => unsafe { self.0.offset(0x28 / 0x4) },
        };

        let pin_level = unsafe { level.offset(register) };
        unsafe { std::ptr::write_volatile(pin_level, 1 << pin_shift) };
        Ok(())
    }
}

fn map_gpio_mem() -> Result<*mut u32> {
    let gpiomem = PathBuf::from("/dev/gpiomem");
    anyhow::ensure!(gpiomem.exists(), "Failed to find /dev/gpiomem");

    let gpiomem = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/gpiomem")
        .with_context(|| "Failed to open /dev/gpiomem")?;

    // FIXME: Why do we only read 4096 bytes?
    // NB: Yes, we're leaking the file descriptor. This is a short lived program and it's more
    // hassle than it's worth to track it's lifetime
    let map = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            4096,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            gpiomem.into_raw_fd(),
            0x0,
        )
    };
    anyhow::ensure!(map != MAP_FAILED, "Failed to mmap /dev/gpiomem");

    Ok(map as *mut u32)
}

fn parse_cells<P: AsRef<Path>>(path: P) -> Result<u32> {
    let mut cells_fd = File::open(&path)
        .with_context(|| format!("Failed to open {} for parsing", path.as_ref().display()))?;

    let mut cells_buf = [0_u8; 4];
    cells_fd
        .read(&mut cells_buf)
        .with_context(|| format!("Failed to parse {} into cells", path.as_ref().display()))?;

    Ok(BigEndian::read_u32(&cells_buf))
}

fn parse_ranges<P: AsRef<std::path::Path>>(
    path: P,
    child_size: u32,
    parent_size: u32,
    length_size: u32,
) -> Result<Vec<(u64, u64, u64)>> {
    let path = path.as_ref();
    anyhow::ensure!(
        child_size == 1 || child_size == 2,
        format!("Invalid child size of {}", child_size)
    );
    anyhow::ensure!(
        parent_size == 1 || parent_size == 2,
        format!("Invalid parent size of {}", parent_size)
    );
    anyhow::ensure!(
        length_size == 1 || length_size == 2,
        format!("Invalid length size of {}", length_size)
    );

    // read the ranges as bytes
    let mut ranges_fd =
        File::open(path).with_context(|| format!("Failed to open ranges {}", path.display()))?;

    let mut ranges: Vec<u8> = Vec::new();
    ranges_fd
        .read_to_end(&mut ranges)
        .with_context(|| format!("Failed to read ranges {}", path.display()))?;

    // parse the bytes into big endian tuples of either u64 or u32, depending on the size info we
    // parsed before.
    let ranges = ranges
        .chunks_exact(((child_size + parent_size + length_size) * 4) as usize)
        .map(|range| {
            let mut range = Vec::from(range);

            let child_addr = match child_size {
                1 => BigEndian::read_u32(&range.drain(0..4).as_slice()) as u64,
                2 => BigEndian::read_u64(&range.drain(0..8).as_slice()),
                _ => unreachable!(),
            };

            let parent_addr = match parent_size {
                1 => BigEndian::read_u32(&range.drain(0..4).as_slice()) as u64,
                2 => BigEndian::read_u64(&range.drain(0..8).as_slice()),
                _ => unreachable!(),
            };

            let length = match length_size {
                1 => BigEndian::read_u32(&range.drain(0..4).as_slice()) as u64,
                2 => BigEndian::read_u64(&range.drain(0..8).as_slice()),
                _ => unreachable!(),
            };
            (child_addr, parent_addr, length)
        })
        .collect();

    Ok(ranges)
}

fn find_host_peripheral_address() -> Result<u64> {
    // 1. find & parse #address-cels, soc/#{address-cells,size-cells}
    let address_cells = parse_cells("/proc/device-tree/#address-cells").unwrap_or_else(|e| {
        warn!(
            "Failed to parse /proc/device-tree/#address-cells due to {}. Using default value of 2",
            e
        );
        2
    });
    debug!("#address-cells = {:#x}", address_cells);

    let soc_address_cells =
        parse_cells("/proc/device-tree/soc/#address-cells").unwrap_or_else(|e| {
            warn!("Failed to parse /proc/device-tree/soc/#address-cells due to {}. Using default value of 2", e);
            2
        });
    debug!("soc/#address-cells = {:#x}", soc_address_cells);

    let soc_size_cells = parse_cells("/proc/device-tree/soc/#size-cells").unwrap_or_else(|e| {
        warn!(
            "Failed to parse /proc/device-tree/soc/#size-cells due to {}. Using default value of 1",
            e
        );
        1
    });
    debug!("soc/#size-cells = {:#x}", soc_size_cells);

    // 2. parse the (child_addr, parent_addr, length) triple from /proc/device-tree/soc/ranges
    // https://readthedocs.org/projects/devicetree-specification/downloads/pdf/stable/
    // p. 15
    let ranges = parse_ranges(
        "/proc/device-tree/soc/ranges",
        soc_address_cells,
        address_cells,
        soc_size_cells,
    )?;
    let (child_addr, parent_addr, length) = ranges
        .into_iter()
        .find(|(child_addr, _, length)| {
            (*child_addr < 0x7e20_0000) && (child_addr + length > 0x7e20_0000)
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to find valid range"))?;
    debug!("child_addr = {:#x}", child_addr);
    debug!("parent_addr = {:#x}", parent_addr);
    debug!("length = {:#x}", length);

    let gpio_addr = (0x7e20_0000 - child_addr) + parent_addr;

    Ok(gpio_addr)
}

fn find_gpio_mem() -> Result<*mut u32> {
    let mem = PathBuf::from("/dev/mem");
    anyhow::ensure!(mem.exists(), "Failed to find /dev/mem");

    let mem = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/mem")
        .with_context(|| "Failed to open /dev/mem")?;

    let bcm_phys_addr = find_host_peripheral_address()
        .with_context(|| "Failed to find host's peripheral address")?;
    debug!("bcm_phys_addr: {:#x}", bcm_phys_addr);

    // FIXME: How/why do we know:
    // 1. that we only need to mmap the first 4096 bytes?
    // 2. That we need to map at offset bcm_phys_addr is clear, but why the 0x20000 offset? Where
    //    is that documented?
    let map = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            4096,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            mem.into_raw_fd(),
            (bcm_phys_addr + 0x20000) as isize,
        )
    };
    anyhow::ensure!(map != MAP_FAILED, "Failed to mmap /dev/mem");

    Ok(map as *mut u32)
}
