//! R17.5: a transfer is one value. It is neither `Copy` nor `Clone`, so
//! giving it back to its buffer uses it up: a second use is E0382.
#![no_std]
#![forbid(unsafe_code)]

pub fn give_back_twice(b: &mut rsk::DmaBuf<16>, t: rsk::DmaTransfer) {
    let _ = b.reclaim(t);
    let _ = b.reclaim(t);
}
