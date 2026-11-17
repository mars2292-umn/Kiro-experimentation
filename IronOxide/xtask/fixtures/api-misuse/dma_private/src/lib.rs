//! R17.5: the bytes of a buffer lent to EasyDMA transfers are reached only
//! through `DmaBuf::bytes`, which gives none while a transfer has them. The
//! field is private (E0616), and the request that borrowed a slice for the
//! length of the call, leaving the bytes reachable during the transfer, no
//! longer exists (E0425).
#![no_std]
#![forbid(unsafe_code)]

pub fn reach(b: &mut rsk::DmaBuf<16>) -> u8 {
    b.bytes[0]
}

pub fn borrow_for_the_call(buf: &[u8]) -> bool {
    rsk::dma_request(0, buf)
}
