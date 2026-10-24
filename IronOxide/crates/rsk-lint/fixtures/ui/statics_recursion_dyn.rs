//@ scope: application
//@ PR-05 statics, PR-17 direct recursion, PR-18 dynamic dispatch and function pointers (and PR-21: `Cell` is a Kernel-only item).
use core::cell::Cell;

static mut COUNTER: u32 = 0;
static FLAG: Cell<u32> = Cell::new(0);
static TABLE: [u8; 4] = [0; 4];
static COUNTER_CELL: rsk::ResourceCell<u32> = rsk::ResourceCell::new(0);

fn fact(n: u32) -> u32 {
    if n == 0 { 1 } else { n * fact(n - 1) }
}

trait Tr {
    fn run(&self);
}

fn call(t: &dyn Tr) {
    t.run();
}

type Handler = fn(u32) -> u32;

struct S;
impl S {
    fn again(&self, n: u32) {
        if n > 0 {
            self.again(n - 1);
        }
    }
    fn other(&self) {
        Self::make();
    }
    fn make() {}
}
