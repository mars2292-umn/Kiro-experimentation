//@ scope: application
//@ PR-16: bounded loops pass; unbounded ones, a dangling label, and a non-constant bound fail.
const N: usize = 8;

fn unbounded(n: usize, tag: &[u8]) {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    for _b in tag {}
    loop {
        i += 1;
    }
}

#[rsk::bounds('scan = N * 2, 'retry = 3)]
fn annotated(n: usize) {
    let mut i = 0;
    'scan: while i < n {
        i += 1;
    }
    'retry: loop {
        if i == 0 {
            break;
        }
        i -= 1;
    }
}

fn derived(n: u32) {
    for _i in 0..N {}
    for _i in 0..=u32::MAX {}
    for _x in [1, 2, 3] {}
    let mut i = 0;
    while i < n && i < 10_000 {
        i += 1;
    }
    while (i < n) && (i <= N as u32) {
        i += 1;
    }
}

fn exempt() -> ! {
    diverge();
    loop {}
}

fn idle() -> ! {
    loop {
        wait_for_interrupt();
    }
}

#[rsk::bounds('nothing = 4)]
fn dangling() {
    for _i in 0..N {}
}

#[rsk::bounds('a = n)]
fn not_constant(n: usize) {
    let mut i = 0;
    'a: while i < n {
        i += 1;
    }
}

fn computed(n: u32) {
    let start = now();
    while now() - start < 8 * 25_000 {
        let _ = n;
    }
}

fn now() -> u32 {
    0
}

fn diverge() -> ! {
    loop {}
}
fn wait_for_interrupt() {}
