#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use kernel::{print, println};

/// The shell main function.
///
/// # Panics
///
/// * If the scancodes queue is uninitialized.
#[allow(clippy::expect_used)]
pub async fn run() {
    loop {
        print!("> ");

        let mut input = String::new();

        let mut parts = input.split_whitespace();
        let Some(command) = parts.next() else {
            continue;
        };
        let args = parts.collect::<Vec<&str>>();

        match command {
            "echo" => {
                let mut output = String::new();

                for arg in args {
                    output.push_str(arg);
                    output.push(' ');
                }

                println!("{output}");
            }
            "exit" => {
                println!("Exiting shell...");
                break;
            }
            _ => {
                println!("{command}: command not found!");
            }
        }
    }
}
