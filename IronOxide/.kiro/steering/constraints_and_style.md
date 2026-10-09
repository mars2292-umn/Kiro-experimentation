## Constraints and style
- `#![no_std]`, no allocator in the kernel, `#![forbid(unsafe_code)]` outside
  a clearly bounded kernel module
- Prefer proofs over tests where feasible; tests are evidence, not guarantees
- Flag every assumption and every open research question explicitly
- Before designing, ask me clarifying questions about target hardware, whether
  multi-core is in scope, and the certification level I eventually want