use core::fmt;

use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

/// The height of the text buffer (normally 25 lines).
const BUFFER_HEIGHT: usize = 25;
/// The width of the text buffer (normally 80 columns).
const BUFFER_WIDTH: usize = 80;

lazy_static! {
    /// A global `Writer` instance that can be used for printing to the VGA text buffer.
    ///
    /// Used by the `print!` and `println!` macros.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

/// The standard color palette in VGA text mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A combination of a foreground and a background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new `ColorCode` with the given foreground and background colors.
    ///
    /// The foreground and background colors are combined to form a single byte.
    /// The first 4 bits are the background color and the last 4 bits are the foreground color.
    ///
    /// # Logic
    /// We shift the background color 4 bits to the left and then OR it with the foreground to get the final color code.
    ///
    /// ### Example
    /// background = 0b0001 (blue), foreground = 0b0010 (green) => 0b00010010 (blue on green)
    ///
    /// ### Formula
    /// (background << 4) | foreground = (0b0001 << 4) | 0b0010 = 0b00010010
    const fn new(foreground: Color, background: Color) -> Self {
        Self((background as u8) << 4 | (foreground as u8))
    }
}

/// A screen character in the VGA text buffer, consisting of an ASCII character and a `ColorCode`.
///
/// The `repr(C)` attribute guarantees that the structs fields are laid out exactly like in a C struct.
///
/// # Fields
///
/// * `ascii_char`: The ASCII character.
/// * `color_code`: The color code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_char: u8,
    color_code: ColorCode,
}

/// A structure representing the VGA text buffer.
///
/// # Fields
///
/// * `chars`: A 2D array of `ScreenChar`s.
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
///
/// Wraps lines at `BUFFER_WIDTH`. Supports newline characters and implements the `core::fmt::Write` trait.
///
/// # Fields
///
/// * `column_position`: The current column position.
/// * `color_code`: The color code.
/// * `buffer`: The buffer.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    ///
    /// # Arguments
    ///
    /// * `byte`: The byte to write.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;

                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_char: byte,
                    color_code,
                });

                self.column_position += 1;
            }
        }
    }

    /// Writes the given ASCII string to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    /// Does **not** support strings with non-ASCII characters, since they can't be printed in the VGA text mode.
    ///
    /// # Arguments
    ///
    /// * `s`: The string to write.
    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII byte or newline.
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // Not part of printable ASCII range.
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// Shifts all lines one line up and clears the last row.
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();

                self.buffer.chars[row - 1][col].write(character);
            }
        }

        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Clears a row by overwriting it with blank characters.
    ///
    /// # Arguments
    ///
    /// * `row`: The row to clear.
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_char: b' ',
            color_code: self.color_code,
        };

        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    /// Writes a string to the VGA text buffer.
    ///
    /// # Arguments
    ///
    /// * `s`: The string to write.
    ///
    /// # Returns
    ///
    /// * `fmt::Result` - The result of the operation.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);

        Ok(())
    }
}

/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Clears the VGA text buffer.
#[macro_export]
macro_rules! clear {
    () => {
        $crate::vga_buffer::_clear()
    };
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
///
/// # Arguments
///
/// * `args`: The arguments to print.
///
/// # Panics
///
/// * If writing to the VGA text buffer fails.
#[allow(clippy::expect_used)]
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    // We need to disable interrupts to avoid a deadlock when the VGA text buffer is used.
    interrupts::without_interrupts(|| {
        WRITER
            .lock()
            .write_fmt(args)
            .expect("Printing to VGA text buffer failed!");
    });
}

/// Clears the VGA text buffer by overwriting it with blank characters.
#[doc(hidden)]
pub fn _clear() {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        for row in 0..BUFFER_HEIGHT {
            writer.clear_row(row);
        }

        writer.column_position = 0;
    });
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

/// Tests that the VGA text buffer is scrolled correctly.
#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

/// Tests that the VGA text buffer is written to correctly.
///
/// # Panics
///
/// * If `writeln!` fails. This can happen if the VGA text buffer is used in an interrupt handler.
#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Some test string that fits on a single line.";
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{s}").expect("writeln failed!");

        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();

            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}

/// Tests that the VGA text buffer colors are set correctly.
///
/// # Panics
///
/// * If the color on screen is not the same as the color set in the test.
#[test_case]
fn test_colors() {
    let foreground = Color::White;
    let background = Color::Black;

    // Test printing.
    let message = "Hello, world!";
    let color_code = ColorCode::new(foreground, background);
    let mut writer = Writer {
        column_position: 0,
        color_code,
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };

    writer.write_string(message);

    // Add an assertion to test the color of the first character.
    let buffer = unsafe { &*(0xb8000 as *const Buffer) };
    let screen_char = buffer.chars[BUFFER_HEIGHT - 1][0].read();

    assert_eq!(screen_char.color_code, color_code);
}
