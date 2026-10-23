//@ scope: kernel
//@ cfg: test and kani code is outside the Flight_Build; feature-gated code is linted.
#[cfg(test)]
mod tests {
    fn t() {
        loop {
            x();
        }
    }
}

#[cfg(kani)]
fn harness(v: &[u8]) {
    for _b in v {}
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
fn target_only(v: &[u8]) {
    for _b in v {}
}

#[cfg(not(all(target_arch = "arm", target_os = "none")))]
fn host_only(v: &[u8]) {
    for _b in v {}
}

fn trace(tag: &[u8]) {
    #[cfg(feature = "trace")]
    {
        for _b in tag {}
    }
}
