use crate::println;

pub async fn print_primes(limit: u32) {
    for i in 0..limit {
        if is_prime(i) {
            println!("{} is prime!", i);
        } else {
            println!("{} is not prime!", i);
        }
    }
}

pub fn is_prime(num: u32) -> bool {
    // If the number is less than 2, it is not prime.
    if num < 2 {
        return false;
    }
    
    for i in 2..num {
        if num % i == 0 {
            return false;
        }
    }
    
    true
}
