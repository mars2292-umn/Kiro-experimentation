//! Rust legacy symbol demangling (`_ZN<len><seg>...<len>h<hash>E`), enough
//! to match a Task entry path of the Task_Model against the binary.

/// `_ZN14p1_demo_part08rsk_jobs7control17h0123456789abcdefE` ->
/// `p1_demo_part0::rsk_jobs::control`. Other names are returned as they are.
pub fn demangle(sym: &str) -> String {
    let Some(rest) = sym.strip_prefix("_ZN") else { return sym.to_string() };
    let bytes = rest.as_bytes();
    let mut i = 0;
    let mut segs: Vec<String> = Vec::new();
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let mut n = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            n = n * 10 + (bytes[i] - b'0') as usize;
            i += 1;
        }
        if i + n > bytes.len() {
            return sym.to_string();
        }
        segs.push(rest[i..i + n].to_string());
        i += n;
    }
    if rest[i..].starts_with('E') {
        if let Some(last) = segs.last() {
            if last.len() == 17 && last.starts_with('h') && last[1..].chars().all(|c| c.is_ascii_hexdigit()) {
                segs.pop();
            }
        }
        segs.iter().map(|s| s.replace("$LT$", "<").replace("$GT$", ">").replace("$u20$", " ").replace("..", "::")).collect::<Vec<_>>().join("::")
    } else {
        sym.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::demangle;

    #[test]
    fn legacy_paths_demangle_and_drop_the_hash() {
        assert_eq!(demangle("_ZN13p1_demo_part08rsk_jobs7control17h0123456789abcdefE"), "p1_demo_part0::rsk_jobs::control");
        assert_eq!(demangle("rsk_irq_entry"), "rsk_irq_entry");
        assert_eq!(demangle("_ZN3foo3barE"), "foo::bar");
    }
}
