use core::fmt;

use cortex_m::delay::Delay;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::prelude::_embedded_hal_blocking_spi_Write;
use embedded_hal::spi::FullDuplex;
use fugit::RateExtU32;
use hal::gpio::Function;
use hal::gpio::FunctionNull;
use hal::gpio::FunctionSio;
use hal::gpio::Pin;
use hal::gpio::PinId;
use hal::gpio::PinState;
use hal::gpio::PullNone;
use hal::gpio::PullType;
use hal::gpio::SioOutput;
use hal::gpio::ValidFunction;
use hal::pac;
use hal::spi::Disabled;
use hal::spi::ValidSpiPinout;
use hal::Spi;
use nb::block;
use rp_pico::hal;

use crate::lambda::DisplayStruct;

pub struct Lcd<P0, P1, P2, S0, V>
where
  P0 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  P1 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  P2 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  S0 : hal::spi::SpiDevice,
  V : hal::spi::ValidSpiPinout<S0>,
{
  lcd_reset : Pin<P0, FunctionSio<SioOutput>, PullNone>,
  chip_select : Pin<P1, FunctionSio<SioOutput>, PullNone>,
  register_select : Pin<P2, FunctionSio<SioOutput>, PullNone>,
  spi : hal::Spi<hal::spi::Enabled, S0, V, 8>,
  current_page : u8,
  current_col : u8,
  inverted_mode : bool,
  line_break_symbol : bool,
}

const WIDTH : u8 = 128;
const HEIGHT : u8 = 8; // 8 * 8 = 64

impl<P0, P1, P2, S0, P> Lcd<P0, P1, P2, S0, P>
where
  P0 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  P1 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  P2 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  S0 : hal::spi::SpiDevice,
  P : ValidSpiPinout<S0>,
{
  pub fn new(
    reset : Pin<P0, impl Function, impl PullType>,
    cs : Pin<P1, impl Function, impl PullType>,
    rs : Pin<P2, impl Function, impl PullType>,
    spi : Spi<Disabled, S0, P, 8>,
    resets : &mut pac::RESETS,
  ) -> Lcd<P0, P1, P2, S0, P> {
    Lcd {
      lcd_reset : reset
        .into_floating_disabled()
        .into_push_pull_output_in_state(PinState::Low),
      chip_select : cs
        .into_floating_disabled()
        .into_push_pull_output_in_state(PinState::High),
      register_select : rs
        .into_floating_disabled()
        .into_push_pull_output_in_state(PinState::High),
      spi : spi.init(
        resets,
        125_000_000u32.Hz(),
        16_000_000u32.Hz(),
        embedded_hal::spi::MODE_3,
      ),
      current_col : 0,
      current_page : 0,
      inverted_mode : false,
      line_break_symbol : true,
    }
  }

  pub fn command_write(&mut self, data : &[u8]) {
    self.chip_select.set_low().unwrap();
    self.register_select.set_low().unwrap();
    self.spi.write(data).unwrap();
    self.register_select.set_high().unwrap();
    self.chip_select.set_high().unwrap();
  }

  #[allow(clippy::cast_possible_truncation)]
  pub fn data_write(&mut self, data : &[u8]) {
    self.chip_select.set_low().unwrap();
    for byte in data {
      if self.inverted_mode {
        block!(self.spi.send(!byte)).unwrap();
        block!(self.spi.read()).unwrap();
      } else {
        block!(self.spi.send(*byte)).unwrap();
        block!(self.spi.read()).unwrap();
      }
    }
    self.current_col = ((self.current_col as usize + data.len()) % 132) as u8;
    self.chip_select.set_high().unwrap();
  }

  pub fn lcd_init(&mut self, delay : &mut Delay) {
    self.lcd_reset.set_high().unwrap();
    delay.delay_ms(6);
    self.command_write(&[0xe2]);
    delay.delay_ms(5);
    self.command_write(&[0x2c]);
    delay.delay_ms(5);
    self.command_write(&[0x2e]);
    delay.delay_ms(5);
    self.command_write(&[0x2f]);
    delay.delay_ms(5);
    self.command_write(&[0x23, 0x81, 0x28, 0xa2, 0xc8, 0xa0, 0x40, 0xaf]);
  }

  pub fn locate(&mut self, page : u8, col : u8) {
    self.command_write(&[0xb0 | (page & 0xf), col & 0xf, 0x10 | (col >> 4)]);
    self.current_col = col;
    self.current_page = page;
  }

  pub fn clear(&mut self) {
    for page in 0..=HEIGHT {
      self.locate(page, 0);
      self.data_write(&[0; WIDTH as usize]);
    }
    self.locate(self.current_page, self.current_col);
  }

  pub fn wrap(&mut self) {
    for _ in self.current_col..128 {
      self.data_write(&[0]);
    }
    self.locate((self.current_page + 1) % HEIGHT, 0);
  }

  #[allow(clippy::cast_possible_truncation)]
  pub fn write_or_wrap(&mut self, data : &[u8]) {
    if self.current_col + data.len() as u8 > WIDTH {
      self.wrap();
      if self.line_break_symbol {
        self.data_write(&[4, 16, 64, 0]);
      }
    }
    self.data_write(data);
    self.data_write(&[0]);
  }

  pub fn write_or_wrap_char(&mut self, c : char) {
    if c == '\n' {
      self.wrap();
    } else if c == DisplayStruct::CURSOR_START {
      self.inverted_mode = true;
      if self.current_col != 0 {
        self.locate(self.current_page, self.current_col - 1);
        self.data_write(&[0]);
      }
    } else if c == DisplayStruct::CURSOR_END {
      self.inverted_mode = false;
    } else {
      self.write_or_wrap(font_map(c));
    }
  }

  pub fn swrite(&mut self, s : &str) {
    for c in s.chars() {
      self.write_or_wrap_char(c);
    }
  }

  pub fn swrite_wrap(&mut self, s : &str) {
    for word in s.split_whitespace() {
      let mut columns = 0;
      for c in word.chars() {
        if c == '\n' {
          break;
        }
        columns += font_map(c).len();
      }
      columns += word.len() - 1; // margins
      let leftover = WIDTH.saturating_sub(self.current_col);
      if columns > leftover as usize * 4 || word.len() <= 4 && columns > (leftover as usize) {
        self.wrap();
      }
      self.swrite(word);
      self.data_write(font_map(' '));
    }
  }
}

impl<P0, P1, P2, S0, V> fmt::Write for Lcd<P0, P1, P2, S0, V>
where
  P0 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  P1 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  P2 : PinId + ValidFunction<FunctionSio<SioOutput>> + ValidFunction<FunctionNull>,
  S0 : hal::spi::SpiDevice,
  V : ValidSpiPinout<S0>,
{
  fn write_str(&mut self, s : &str) -> fmt::Result {
    self.swrite(s);
    Ok(())
  }

  fn write_char(&mut self, c : char) -> fmt::Result {
    self.write_or_wrap_char(c);
    Ok(())
  }

  #[inline]
  fn write_fmt(mut self: &mut Self, args : fmt::Arguments<'_>) -> fmt::Result {
    fmt::write(&mut self, args)
  }
}

const FONT_DATA_LEN : usize = 124;

static FONT_DATA : [&[u8]; FONT_DATA_LEN] = [
  &[95],
  &[7, 0, 7],
  &[36, 126, 36, 126, 36],
  &[36, 42, 127, 42, 18],
  &[67, 51, 8, 102, 97],
  &[32, 86, 73, 54, 80],
  &[7],
  &[62, 65, 65],
  &[65, 65, 62],
  &[42, 28, 8, 28, 42],
  &[8, 8, 62, 8, 8],
  &[176, 112],
  &[8, 8, 8, 8, 8],
  &[96, 96],
  &[64, 48, 8, 6, 1],
  &[62, 81, 73, 69, 62],
  &[66, 127, 64],
  &[66, 97, 81, 73, 70],
  &[34, 73, 73, 73, 54],
  &[24, 20, 18, 127, 16],
  &[39, 69, 69, 69, 57],
  &[60, 74, 73, 73, 48],
  &[1, 113, 9, 5, 3],
  &[54, 73, 73, 73, 54],
  &[6, 73, 73, 41, 30],
  &[54, 54],
  &[182, 118],
  &[8, 20, 20, 34, 34],
  &[20, 20, 20, 20, 20],
  &[34, 34, 20, 20, 8],
  &[2, 1, 81, 9, 6],
  &[62, 65, 93, 85, 93, 81, 30],
  &[126, 9, 9, 9, 126],
  &[127, 73, 73, 73, 54],
  &[62, 65, 65, 65, 34],
  &[127, 65, 65, 65, 62],
  &[127, 73, 73, 73, 73],
  &[127, 9, 9, 9, 1],
  &[62, 65, 65, 73, 58],
  &[127, 8, 8, 8, 127],
  &[65, 127, 65],
  &[32, 64, 65, 63],
  &[127, 8, 20, 34, 65],
  &[127, 64, 64, 64, 64],
  &[127, 2, 4, 8, 4, 2, 127],
  &[127, 4, 8, 16, 127],
  &[62, 65, 65, 65, 62],
  &[127, 9, 9, 9, 6],
  &[62, 65, 81, 33, 94],
  &[127, 9, 25, 41, 70],
  &[38, 73, 73, 73, 50],
  &[1, 1, 127, 1, 1],
  &[63, 64, 64, 64, 63],
  &[31, 32, 64, 32, 31],
  &[63, 64, 64, 63, 64, 64, 63],
  &[99, 20, 8, 20, 99],
  &[3, 4, 120, 4, 3],
  &[97, 81, 73, 69, 67],
  &[127, 65, 65],
  &[1, 6, 8, 48, 64],
  &[65, 65, 127],
  &[4, 2, 127, 2, 4],
  &[64, 64, 64, 64, 64],
  &[1, 2],
  &[32, 84, 84, 84, 120],
  &[127, 68, 68, 68, 56],
  &[56, 68, 68, 68, 40],
  &[56, 68, 68, 68, 127],
  &[56, 84, 84, 84, 24],
  &[8, 126, 9, 2],
  &[24, 164, 164, 164, 124],
  &[127, 4, 4, 4, 120],
  &[4, 61, 64, 64],
  &[64, 132, 125],
  &[127, 16, 40, 68],
  &[1, 63, 64, 64],
  &[124, 4, 4, 120, 4, 4, 120],
  &[124, 8, 4, 4, 120],
  &[56, 68, 68, 68, 56],
  &[252, 36, 36, 36, 24],
  &[24, 36, 36, 36, 252],
  &[124, 8, 4, 4, 4],
  &[72, 84, 84, 84, 36],
  &[4, 63, 68, 64],
  &[60, 64, 64, 64, 124],
  &[28, 32, 64, 32, 28],
  &[60, 64, 48, 64, 60],
  &[68, 40, 16, 40, 68],
  &[28, 160, 160, 160, 124],
  &[68, 100, 84, 76, 68],
  &[8, 54, 65, 65],
  &[127],
  &[65, 65, 54, 8],
  &[8, 4, 8, 16, 8],
  &[8, 20, 42, 20, 34],
  &[252, 64, 64, 64, 60, 64],
  &[34, 20, 42, 20, 8],
  &[112, 12, 3, 12, 112],
  &[96, 25, 6, 24, 96],
  &[56, 68, 64, 32, 64, 68, 56],
  &[28, 28, 28],
  &[99, 65, 36, 62, 32, 65, 99],
  &[99, 65, 38, 50, 46, 65, 99],
  &[99, 65, 42, 42, 62, 65, 99],
  &[99, 65, 24, 20, 62, 81, 99],
  &[99, 65, 46, 42, 58, 65, 99],
  &[99, 65, 28, 42, 16, 65, 99],
  &[99, 65, 50, 10, 6, 65, 99],
  &[99, 65, 62, 42, 62, 65, 99],
  &[99, 65, 46, 42, 30, 65, 99],
  &[65, 62, 0, 62, 34, 62, 65],
  &[62, 127, 91, 65, 95, 127, 62],
  &[62, 127, 89, 77, 81, 127, 62],
  &[62, 127, 85, 85, 65, 127, 62],
  &[62, 127, 103, 107, 65, 111, 62],
  &[62, 127, 81, 85, 69, 127, 62],
  &[62, 127, 99, 85, 103, 127, 62],
  &[62, 127, 77, 117, 121, 127, 62],
  &[62, 127, 65, 85, 65, 127, 62],
  &[62, 127, 81, 85, 97, 127, 62],
  &[62, 65, 127, 65, 93, 65, 62],
  &[62, 127, 127, 127, 127, 127, 62],
  &[99, 65, 62, 34, 62, 65, 99],
  &[62, 127, 65, 93, 65, 127, 62],
];

pub(crate) static CHAR_LIST : [char; FONT_DATA_LEN] = [
  '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/', '0', '1', '2', '3',
  '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?', '@', 'A', 'B', 'C', 'D', 'E', 'F',
  'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y',
  'Z', '[', '\\', ']', '^', '_', '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
  'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '«',
  'µ', '»', 'Λ', 'λ', 'ω', '▪', '➀', '➁', '➂', '➃', '➄', '➅', '➆', '➇', '➈', '➉', '➊', '➋', '➌',
  '➍', '➎', '➏', '➐', '➑', '➒', '➓', '⬤', '🄋', '🄌',
];

#[must_use]
pub fn font_map(c : char) -> &'static [u8] {
  if c == ' ' {
    &[0; 3]
  } else if c == '\n' || c == DisplayStruct::CURSOR_END || c == DisplayStruct::CURSOR_START {
    &[]
  } else if let Ok(pos) = CHAR_LIST.binary_search(&c) {
    unsafe { FONT_DATA.get_unchecked(pos) }
  } else {
    &[85, 42, 85, 42, 85]
  }
}
