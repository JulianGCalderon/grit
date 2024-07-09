use std::ops::{BitAnd, Shr};

/// Applies mask to original number and shifts it right
///
/// ## Example
///
/// Given the following arguments:
///
/// - number: 0x1010101
/// - mask:   0x0011100
/// - shift:  2
///
/// Then this function will return 0x101
pub fn extract_bits<N>(number: N, mask: N, shift: u8) -> N
where
    N: BitAnd<Output = N>,
    N: Shr<u8, Output = N>,
{
    (number & mask) >> shift
}
