//! R6.7: the Kernel's version identifier and the Profile version and
//! Profile_Variant it implements, as linked symbols.
//!
//! Each is a fixed-length byte string, zero-padded, that the Config_Checker
//! reads from the binary by name and compares with the Task_Model
//! (`kernel`, `profile.version`, `profile.variant`): a binary linked with
//! another Kernel than the one the model was generated for, or with none
//! of these symbols, is reported. Nothing in the Kernel reads them: they
//! lie in the section `.rsk.version`, which the generated linker script
//! keeps in the Kernel's flash region. The statics themselves are in the
//! Kernel_Unsafe_Module (`arch::cm4::unsafe_module`), since a symbol name
//! and a section are link-level declarations (`unsafe_code`).
#![forbid(unsafe_code)]

/// The Kernel's version identifier.
pub const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The length of each version symbol in bytes.
pub const VERSION_SYMBOL_LEN: usize = 16;

/// `s` as a zero-padded byte string. A string longer than the symbol does
/// not compile.
pub const fn version_bytes(s: &str) -> [u8; VERSION_SYMBOL_LEN] {
    let bytes = s.as_bytes();
    assert!(bytes.len() <= VERSION_SYMBOL_LEN);
    let mut out = [0u8; VERSION_SYMBOL_LEN];
    let mut i = 0;
    while i < VERSION_SYMBOL_LEN {
        if i < bytes.len() {
            out[i] = bytes[i];
        }
        i += 1;
    }
    out
}
