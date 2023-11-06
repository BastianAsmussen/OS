use alloc::string::String;
use core::pin::Pin;
use core::task::{Context, Poll};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::task::AtomicWaker;
use futures_util::{FutureExt, Stream, StreamExt};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

use crate::print;
use crate::println;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
///
/// # Arguments
///
/// * `scancode` - The scancode received from the keyboard.
pub(crate) fn add_scancode(scancode: u8) {
    SCANCODE_QUEUE.try_get().map_or_else(
        |_| {
            println!("Warning: Scancode queue uninitialized!");
        },
        |queue| {
            if queue.push(scancode).is_err() {
                println!("Warning: Scancode queue full; dropping keyboard input!");
            } else {
                WAKER.wake();
            }
        },
    );
}

/// A stream of scancodes received from the keyboard.
///
/// # Fields
///
/// * `_private` - A private field to prevent construction outside of this module.
#[derive(Clone, Copy)]
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    /// Creates a new `ScancodeStream`.
    ///
    /// # Panics
    ///
    /// * If called more than once.
    #[allow(clippy::expect_used)]
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once!");

        Self { _private: () }
    }

    #[must_use]
    pub const fn get_active_stream() -> Self {
        Self { _private: () }
    }
}

impl Default for ScancodeStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    /// Polls the stream for the next scancode.
    ///
    /// # Arguments
    ///
    /// * `cx` - The context to use for polling.
    ///
    /// # Returns
    ///
    /// * `Poll<Option<u8>>` - The next scancode, if available.
    ///
    /// # Panics
    ///
    /// * If the scancode queue is not initialized.
    /// * If the waker is already set.
    /// * If the waker is not set.
    #[allow(clippy::expect_used)]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("Scancode queue not initialized!");

        // Fast path if we have already received a scancode.
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

/// Reads a line from the keyboard.
///
/// # Arguments
///
/// * `s` - The string to read into.
pub async fn read_line(s: &mut String) {
    let mut scancode_stream = ScancodeStream::get_active_stream();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::MapLettersToUnicode,
    );

    loop {
        let scancode = scancode_stream.next().await;

        if let Some(scancode) = scancode {
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(character) => {
                            if character == '\n' {
                                println!();
                                break;
                            }

                            s.push(character);
                            print!("{}", character);
                        }
                        DecodedKey::RawKey(key) => {
                            if key == pc_keyboard::KeyCode::Backspace && !s.is_empty() {
                                s.pop();
                                print!("\u{8} \u{8}");
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Reads a line from the keyboard.
///
/// # Arguments
///
/// * `s` - The string to read into.
///
/// # Panics
///
/// * If the waker is already set.
/// * If the waker is not set.
pub fn read_line_blocking(s: &mut String) {
    let mut scancode_stream = ScancodeStream::get_active_stream();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::MapLettersToUnicode,
    );

    let waker = WAKER.take().expect("Waker not set!");

    loop {
        let mut scancode = scancode_stream
            .next()
            .poll_unpin(&mut Context::from_waker(&waker));

        while scancode == Poll::Pending {
            scancode_stream = ScancodeStream::get_active_stream();
            scancode = scancode_stream
                .next()
                .poll_unpin(&mut Context::from_waker(&waker));
        }

        if let Poll::Ready(Some(scancode)) = scancode {
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(character) => {
                            if character == '\n' {
                                println!();
                                break;
                            }

                            s.push(character);
                            print!("{}", character);
                        }
                        DecodedKey::RawKey(key) => {
                            if key == pc_keyboard::KeyCode::Backspace && !s.is_empty() {
                                s.pop();
                                print!("\u{8} \u{8}");
                            }
                        }
                    }
                }
            }
        }
    }
}

pub async fn print_keypress() {
    let mut scancode_stream = ScancodeStream::get_active_stream();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::MapLettersToUnicode,
    );

    loop {
        let scancode = scancode_stream.next().await;

        if let Some(scancode) = scancode {
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(character) => {
                            print!("{}", character);
                        }
                        DecodedKey::RawKey(key) => {
                            if key == pc_keyboard::KeyCode::Backspace {
                                print!("\u{8} \u{8}");
                            }
                        }
                    }
                }
            }
        }
    }
}
