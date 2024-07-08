use std::ops::{BitAnd, Shr};

// only receiving mask is probably simpler to read
pub fn extract_bits<N>(original: N, mask: N, location: u8) -> N
where
    N: Shr<u8, Output = N>,
    N: BitAnd<Output = N>,
{
    (original >> location) & mask
}
