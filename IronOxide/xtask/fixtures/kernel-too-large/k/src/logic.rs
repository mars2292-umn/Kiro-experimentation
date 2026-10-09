#![forbid(unsafe_code)]

/// Three executable lines (the closing brace is layout).
pub fn f(a: u32) -> u32 {
    let b = a.wrapping_add(1);
    b.wrapping_mul(2)
}
