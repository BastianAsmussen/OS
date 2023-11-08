use crate::println;

/// Prints all prime numbers up to the given limit.
///
/// # Arguments
///
/// * `limit` - The upper limit of primes to print.
#[allow(clippy::module_name_repetitions)]
pub fn print_primes(limit: u32) {
    for i in 0..limit {
        if is_prime(i) {
            println!("{} is prime!", i);
        } else {
            println!("{} is not prime!", i);
        }
    }
}

/// Checks if the given number is prime.
///
/// # Arguments
///
/// * `num` - The number to check.
///
/// # Returns
///
/// * `bool` - Whether or not the number is prime.
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn is_prime(num: u32) -> bool {
    (2..num).all(|i| num % i != 0)
}
