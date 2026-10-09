//! Safe code, but the crate root does not forbid `unsafe_code`.

pub fn double(x: u32) -> Option<u32> {
    x.checked_mul(2)
}
