#![no_std]
#![no_main]
#![feature(prelude_2024, box_patterns, let_chains)]

extern crate alloc;

pub mod lambda;
pub mod lcd;

use alloc_cortex_m::CortexMHeap;
use core::fmt::Write;
use core::mem;
use cortex_m::asm::wfi;
#[global_allocator]
static ALLOCATOR : CortexMHeap = CortexMHeap::empty();

use hal::gpio::FunctionSpi;
use panic_probe as _;
use rp_pico::entry;
use rp_pico::hal;
use rtt_target::rprintln;

use hal::{
  clocks::{init_clocks_and_plls, Clock},
  pac,
  sio::Sio,
  watchdog::Watchdog,
};

use crate::lambda::Expr;
use crate::lambda::Expr::*;
use crate::lambda::LeafMode;
use crate::lambda::{app, lam, PLUS, POWER, TIMES};

#[entry]
fn main() -> ! {
  rtt_target::rtt_init_print!();
  rprintln!("Program start");

  {
    // use core::mem::MaybeUninit;
    // const HEAP_SIZE : usize = 128 * 1024;
    // #[link_section = ".uninit"]
    // static mut HEAP_MEM : [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit();
    // HEAP_SIZE]; unsafe { ALLOCATOR.init(HEAP_MEM.as_ptr() as usize,
    // HEAP_SIZE) }
    unsafe {
      let ptr = cortex_m_rt::heap_start() as usize;
      ALLOCATOR.init(ptr, 0x2004_0000 - ptr);
    }
  }

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
  let timer = hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS);

  let pins = hal::gpio::Pins::new(
    pac.IO_BANK0,
    pac.PADS_BANK0,
    sio.gpio_bank0,
    &mut pac.RESETS,
  );

  // let mut led_pin = pins.gpio25.into_push_pull_output();

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

  // let mut rtt_input_buf = [0u8; 256];
  loop {
    let input =
      // "^ 2 4 redux b dn dn dn dn dn dn dn rt rt rt rt rt rt up up rt up up rm";
      "l l . 1 up up up";
    let mut cursor = Hole;
    let mut expr = Slot;
    let mut leaf_mode = LeafMode::No;
    for cmd in input.split_whitespace() {
      let start = timer.get_counter_low();
      lcd.clear();
      lcd.locate(0, 0);
      writeln!(lcd, "cmd={cmd}").unwrap();
      match cmd {
        "bs" => cursor = Hole,
        "l" => {
          if cursor == Hole {
            expr.replace_slot(lam(Slot));
          } else {
            cursor = lam(cursor);
          }
        }
        "b" => {
          if !cursor.beta() {
            rprintln!("boop(beta)");
          }
        }
        "redux" => {
          if let Some(hd) = cursor.find_redux() {
            let mut new = Slot;
            mem::swap(hd, &mut new);
            expr.replace_slot(cursor);
            cursor = new;
          } else {
            rprintln!("boop(redux)");
          }
        }
        "dn" => match cursor {
          Lam(e) => {
            expr.replace_slot(lam(Slot));
            cursor = *e;
          }
          App(l, r) => {
            expr.replace_slot(app(Slot, *r));
            cursor = *l;
          }
          Hole | Var(_) => {
            if leaf_mode == LeafMode::Leaf {
              rprintln!("boop");
            } else {
              leaf_mode = LeafMode::Leaf;
            }
          }
          Slot => panic!(),
        },
        "up" => {
          if leaf_mode == LeafMode::Leaf {
            leaf_mode = LeafMode::No;
          } else if let Some(p) = expr.find_slot_parent() {
            if let App(box Slot, box e) | App(box e, box Slot) = p
            && cursor == Hole {
              mem::swap(e, &mut cursor);
              *p = Slot;
            } else {
              let mut new = Slot;
              mem::swap(p, &mut new);
              new.replace_slot(cursor);
              cursor = new;
            }
          } else {
            rprintln!("boop");
          }
        }
        "top" => {
          expr.replace_slot(cursor);
          cursor = expr;
          expr = Slot;
        }
        "lm" => {
          let mut new = Slot;
          mem::swap(cursor.leftmost(), &mut new);
          expr.replace_slot(cursor);
          cursor = new;
          leaf_mode = LeafMode::Leaf;
        }
        "rm" => {
          let mut new = Slot;
          mem::swap(cursor.rightmost(), &mut new);
          expr.replace_slot(cursor);
          cursor = new;
          leaf_mode = LeafMode::Leaf;
        }
        "lt" => {
          if leaf_mode == LeafMode::Leaf {
            if let Some((slot, sib)) = expr.find_slot_leftsib() {
              mem::swap(slot, &mut cursor);
              mem::swap(sib, &mut cursor);
            } else {
              leaf_mode = LeafMode::No;
            }
          } else if let Some(p) = expr.find_slot_parent() {
            match p {
              App(box e, box Slot) if cursor == Hole => {
                mem::swap(e, &mut cursor);
                *p = Slot;
              }
              App(box l, box r) if *r == Slot => {
                mem::swap(r, &mut cursor);
                mem::swap(l, &mut cursor);
              }
              _ => {
                let mut new = Slot;
                mem::swap(p, &mut new);
                new.replace_slot(cursor);
                cursor = new;
              }
            }
          } else {
            rprintln!("boop");
          }
        }
        "rt" => {
          if leaf_mode == LeafMode::Leaf {
            if let Some((slot, sib)) = expr.find_slot_rightsib() {
              mem::swap(slot, &mut cursor);
              mem::swap(sib, &mut cursor);
            } else {
              leaf_mode = LeafMode::No;
            }
          } else if let Some(p) = expr.find_slot_parent() {
            match p {
              App(box Slot, box e) if cursor == Hole => {
                mem::swap(e, &mut cursor);
                *p = Slot;
              }
              App(l, r) if **l == Slot => {
                mem::swap::<Expr>(l, &mut cursor);
                mem::swap::<Expr>(r, &mut cursor);
              }
              _ => {
                let mut new = Slot;
                mem::swap(p, &mut new);
                new.replace_slot(cursor);
                cursor = new;
              }
            }
          } else {
            rprintln!("boop");
          }
        }
        "$" => {
          expr.replace_slot(app(cursor, Slot));
          cursor = Hole;
        }
        "@" => {
          expr.replace_slot(app(Slot, cursor));
          cursor = Hole;
        }
        "+" => match cursor {
          Hole => cursor = PLUS.clone(),
          _ => cursor = app(PLUS.clone(), cursor),
        },
        "*" => match cursor {
          Hole => cursor = TIMES.clone(),
          _ => cursor = app(TIMES.clone(), cursor),
        },
        "^" => match cursor {
          Hole => cursor = POWER.clone(),
          _ => cursor = app(POWER.clone(), cursor),
        },
        "." => {
          if cursor == Hole {
            leaf_mode = LeafMode::InputDot;
          } else {
            cursor = app(cursor, Slot);
            expr.replace_slot(cursor);
            cursor = Hole;
            leaf_mode = LeafMode::InputDot;
          }
        }
        s => {
          if let Ok(u) = s.parse() {
            if cursor == Hole {
              if leaf_mode == LeafMode::InputDot {
                cursor = Var(u);
                leaf_mode = LeafMode::Leaf;
              } else {
                cursor = Expr::from_nat(u);
              }
            } else {
              cursor = app(cursor, Expr::from_nat(u));
            }
          } else if let Some(u) = s
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .and_then(|s| s.parse().ok())
          {
            if cursor == Hole {
              cursor = Var(u);
            } else {
              cursor = app(cursor, Var(u));
            }
          } else {
            rprintln!("unrec'd cmd: {s}");
          }
        }
      }
      // for c in lcd::CHAR_LIST {
      //   lcd.write_or_wrap(lcd::font_map(*c), false);
      //   lcd.data_write(&[0]);
      // }
      // lcd.wrap();
      // let mut pow24 = app(
      //   app(
      //     lambda::POWER.clone(),
      //     app(
      //       app(lambda::PLUS.clone(), Expr::from_nat(1)),
      //       Expr::from_nat(1),
      //     ),
      //   ),
      //   Expr::from_nat(6),
      // );
      // let mut max_use = 0;
      // while let Some(r) = pow24.find_redux() {
      //   max_use = usize::max(max_use, ALLOCATOR.used());
      //   r.beta();
      //   writeln!(lcd, "=> {pow24}").unwrap();
      // }
      lcd.data_write(&[0]);
      writeln!(
        lcd,
        "{}",
        lambda::DisplayStruct {
          expr : &expr,
          cursor : &cursor,
          leaf_mode
        }
      )
      .unwrap();
      writeln!(lcd, "free={}", ALLOCATOR.free()).unwrap();
      writeln!(lcd, "used={}", ALLOCATOR.used()).unwrap();
      writeln!(lcd, "time={}", (timer.get_counter_low() - start)).unwrap();
      // rprintln!("on!");
      // led_pin.set_high().unwrap();
      // delay.delay_ms(500);
      // rprintln!("off!");
      // led_pin.set_low().unwrap();
      delay.delay_ms(1000);
    }
  }
}
