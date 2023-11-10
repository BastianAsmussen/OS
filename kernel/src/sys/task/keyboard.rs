use core::pin::Pin;
use core::task::{Context, Poll};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::task::AtomicWaker;
use futures_util::{Stream, StreamExt};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

use crate::print;
use crate::println;

/// The scancode queue.
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
/// The waker.
///
/// This is used to wake up the `read_line` function when a scancode is received.
static WAKER: AtomicWaker = AtomicWaker::new();

/// The size of the scancode queue.
const SCANCODE_QUEUE_SIZE: usize = 100;

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
///
/// # Arguments
///
/// * `scancode` - The scancode received from the keyboard.
///
/// # Panics
///
/// * If the scancode queue is not initialized.
/// * If the scancode queue is full.
pub(crate) fn add_scancode(scancode: u8) {
    if SCANCODE_QUEUE
        .get_or_init(|| ArrayQueue::new(SCANCODE_QUEUE_SIZE))
        .push(scancode)
        .is_err()
    {
        println!("[WARN]: Scancode queue full, dropping keyboard input...");
    }

    WAKER.wake();
}

/// An API for interacting with the [`SCANCODE_QUEUE`].
#[derive(Clone, Copy)]
pub struct ScancodeStream;

impl ScancodeStream {
    /// Creates a new [`ScancodeStream`] instance for interacting with the [`SCANCODE_QUEUE`].
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Stream for ScancodeStream {
    /// The type of item produced by the stream.
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
    /// # Notes
    ///
    /// * If the [`SCANCODE_QUEUE`] is uninitialized, it will be initialized with a capacity of [`SCANCODE_QUEUE_SIZE`].
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE.get_or_init(|| ArrayQueue::new(SCANCODE_QUEUE_SIZE));

        // Fast path if we have already received a scancode.
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        queue.pop().map_or(Poll::Pending, |scancode| {
            WAKER.take();

            Poll::Ready(Some(scancode))
        })
    }
}

/// Print keys pressed on the keyboard.
pub async fn print_keypress() {
    let mut scancode_stream = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::MapLettersToUnicode,
    );

    while let Some(scancode) = scancode_stream.next().await {
        let Ok(Some(key_event)) = keyboard.add_byte(scancode) else {
            continue;
        };
        let Some(key) = keyboard.process_keyevent(key_event) else {
            continue;
        };

        match key {
            DecodedKey::Unicode(character) => print!("{character}"),
            DecodedKey::RawKey(key) => print!("{key:?}"),
        }
    }
}
