use core::pin::Pin;
use core::task::{Context, Poll};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::stream::StreamExt;
use futures_util::task::AtomicWaker;
use futures_util::Stream;
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

pub async fn print_keypress() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
