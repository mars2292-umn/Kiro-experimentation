//@ scope: application
//@ A Task body that uses every construct the lint admits: no finding.
use core::marker::PhantomData;

const LIMIT: u32 = 100;

struct Ctx<'a> {
    marker: PhantomData<&'a ()>,
}

#[rsk::bounds('samples = LIMIT / 2)]
pub fn control(cx: Ctx<'_>) {
    let mut acc = 0u32;
    for i in 0..LIMIT {
        acc = acc.wrapping_add(i);
    }
    let mut k = 0;
    'samples: while k < acc && k < LIMIT {
        k += 1;
    }
    let _ = (cx, core::cmp::max(acc, k));
}
