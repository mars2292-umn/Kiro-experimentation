//@ scope: application
//@ R35.2: an App_Prover variant on a loop must state the loop's Loop_Bound_Annotation; a different constant, or none, is reported.
const N: usize = 8;

fn agreeing(n: u32) {
    let mut i = 0u32;
    #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(i@ <= 10_000))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::variant(10_000 - i@))]
    while i < n && i < 10_000 {
        i += 1;
    }
    #[variant(N - i@)]
    for _i in 0..N {}
}

#[rsk::bounds('scan = N * 2)]
fn annotated(n: usize) {
    let mut i = 0;
    #[cfg_attr(creusot, rsk::creusot_std::macros::variant(N * 2 - i@))]
    'scan: while i < n {
        i += 1;
    }
}

fn disagreeing(n: u32) {
    let mut i = 0u32;
    #[cfg_attr(creusot, rsk::creusot_std::macros::variant(9_999 - i@))]
    while i < n && i < 10_000 {
        i += 1;
    }
    #[cfg_attr(creusot, rsk::creusot_std::macros::variant(n@ - i@))]
    while i < n && i < 10_000 {
        i += 1;
    }
}

fn unproven(n: u32) {
    let mut i = 0u32;
    while i < n && i < 10_000 {
        i += 1;
    }
}
