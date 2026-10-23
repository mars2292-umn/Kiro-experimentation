//@ scope: kernel
//@ PR-16 inside verus!: a loop without `decreases` fails; `decreases`, a range `for`, and `impl ... for` pass.
verus! {

impl<T> Tr for S<T> {
    fn ok(&self, n: usize) {
        let mut i = 0;
        while i < n
            invariant i <= n,
            decreases n - i,
        {
            i += 1;
        }
        for j in 0..n
            invariant j <= n,
        {
        }
    }

    fn bad(&self, n: usize) {
        let mut i = 0;
        while i < n
            invariant i <= n,
        {
            i += 1;
        }
    }
}

} // verus!
