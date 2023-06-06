#![no_std]
#![no_main]
#![feature(prelude_2024, box_patterns, let_chains)]

extern crate alloc;

pub mod lambda;
pub mod lcd;

use alloc_cortex_m::CortexMHeap;
#[global_allocator]
static ALLOCATOR : CortexMHeap = CortexMHeap::empty();

use embedded_hal::digital::v2::OutputPin;
use hal::gpio::FunctionSpi;
use panic_probe as _;
use rp_pico::entry;
use rp_pico::hal;
use rtt_target::rprintln;
use rtt_target::rtt_init_print;

use hal::{
  clocks::{init_clocks_and_plls, Clock},
  pac,
  sio::Sio,
  watchdog::Watchdog,
};

#[entry]
fn main() -> ! {
  rtt_init_print!();
  rprintln!("Program start");
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

  let _ = pins.gpio2.into_mode::<FunctionSpi>();
  let _ = pins.gpio3.into_mode::<FunctionSpi>();
  let _ = pins.gpio4.into_mode::<FunctionSpi>();

  let mut lcd = lcd::Lcd::new(
    pins.gpio0,
    pins.gpio5,
    pins.gpio1,
    hal::spi::Spi::<_, _, 8>::new(pac.SPI0),
    &mut pac.RESETS,
  );

  lcd.lcd_init(&mut delay);
  lcd.clear();

  lcd.locate(0, 0);
  for c in lcd::CHAR_LIST {
    lcd.write_or_wrap(lcd::font_map(*c), false);
    lcd.data_write(&[0]);
  }
  lcd.swrite_wrap("On the other hand, we denounce with righteous indignation and dislike men who are so beguiled and demoralized by the charms of pleasure of the moment, so blinded by desire");

  loop {
    rprintln!("on!");
    led_pin.set_high().unwrap();
    delay.delay_ms(500);
    rprintln!("off!");
    led_pin.set_low().unwrap();
    delay.delay_ms(500);
  }
}
