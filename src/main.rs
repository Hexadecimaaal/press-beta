#![no_std]
#![no_main]
#![feature(prelude_2024, box_patterns, let_chains, thread_local)]

extern crate alloc;

pub mod lambda;

use alloc_cortex_m::CortexMHeap;
#[global_allocator]
static ALLOCATOR : CortexMHeap = CortexMHeap::empty();

use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use panic_probe as _;
use rp2040_hal as hal;

use hal::{
  clocks::{init_clocks_and_plls, Clock},
  sio::Sio,
  pac,
  watchdog::Watchdog,
};

#[link_section = ".boot2"]
#[used]
pub static BOOT2 : [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
  info!("Program start");
  let mut pac = pac::Peripherals::take().unwrap();
  let core = pac::CorePeripherals::take().unwrap();
  let mut watchdog = Watchdog::new(pac.WATCHDOG);
  let sio = Sio::new(pac.SIO);

  // External high-speed crystal on the pico board is 12Mhz
  let external_xtal_freq_hz = 12_000_000u32;
  let clocks = init_clocks_and_plls(
    external_xtal_freq_hz,
    pac.XOSC,
    pac.CLOCKS,
    pac.PLL_SYS,
    pac.PLL_USB,
    &mut pac.RESETS,
    &mut watchdog,
  )
  .ok()
  .unwrap();

  let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

  let pins = hal::gpio::Pins::new(
    pac.IO_BANK0,
    pac.PADS_BANK0,
    sio.gpio_bank0,
    &mut pac.RESETS,
  );

  let mut led_pin = pins.gpio25.into_push_pull_output();

  loop {
    info!("on!");
    led_pin.set_high().unwrap();
    delay.delay_ms(500);
    info!("off!");
    led_pin.set_low().unwrap();
    delay.delay_ms(500);
  }
}
