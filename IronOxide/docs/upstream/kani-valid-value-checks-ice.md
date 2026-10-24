# Kani: internal compiler error with `-Z valid-value-checks` on slice iteration

**Already reported and fixed upstream (checked 2026-10-23).** This is
model-checking/kani issue #4829, "panic: `-Z valid-value-checks` ICEs on
any allocating function: fat-pointer pattern type in `NonNull<[u8]>`", opened
2026-09-22 and closed 2026-09-30 by PR #4856 ("Fix ICE on pattern types over
wide pointers under -Z valid-value-checks"). Same assertion
(`check_values.rs:959`, "expected pattern type to have a scalar ABI"), same
cause (the non-null pattern over a wide slice pointer has a `ScalarPair`
layout, not a scalar). The fix is on `main` but in no release: the latest
release is kani-0.68.0 of 2026-09-16, the version pinned here. Nothing to
file; the invalid-value checks return to `rsk-kernel` when a release past
0.68.0 is pinned (toolchain/decision.toml [kani]). Issue #5056 (spurious
"Invalid value" reports during initialization, open) is a different defect of
the same experimental check and may affect `rsk-kernel`'s cells once the
check runs; to be watched at that upgrade.

The text below is the reproduction as found here, kept as the record behind
Verification_Plan section 6.1.

## Summary

With `-Z valid-value-checks`, `kani-compiler` panics in
`kani_middle/transform/check_values.rs:959` ("expected pattern type to have
a scalar ABI") on any harness that iterates a slice. The pattern type is
the non-null raw slice pointer that `core::slice::Iter` holds
(`NonNull<[T]>`, a `RawPtr(Slice(..))` with the `NotNull` pattern), which
has no scalar ABI. Without the flag the same harness verifies.

## Versions

- Kani 0.68.0 (cargo plugin), CBMC 6.11.0
- macOS, aarch64-apple-darwin
- Found on a 6-harness crate of the rsk kernel (`rsk-kernel`), reproduced
  with the minimal crate below

## Minimal reproduction

`Cargo.toml`:

```toml
[package]
name = "kani-ice"
version = "0.0.0"
edition = "2021"
```

`src/lib.rs`:

```rust
#[cfg(kani)]
#[kani::proof]
fn sum_of_slice_iter() {
    let a: [u32; 3] = [kani::any(), 1, 2];
    kani::assume(a[0] < 10);
    let s: u32 = a.iter().copied().sum();
    assert!(s >= 3);
}
```

```
$ cargo kani                          # verifies
$ cargo kani -Z valid-value-checks    # panics
thread 'rustc' panicked at kani-compiler/src/kani_middle/transform/check_values.rs:959:9:
error: internal compiler error: Kani unexpectedly panicked at panicked at kani-compiler/src/kani_middle/transform/check_values.rs:959:9:
    expected pattern type to have a scalar ABI: Ty { id: 161, kind: RigidTy(Pat(Ty { id: 160, kind: RigidTy(RawPtr(Ty { id: 17, kind: RigidTy(Slice(Ty { id: 2, kind: RigidTy(Uint(U32)) })) }, Not)) }, NotNull)) }
```

## Expected

Either the pattern type on a wide (slice) pointer is handled by the
valid-value transform, treating the `NotNull` pattern as a check on the
data pointer alone, or the transform skips the type with a diagnostic
instead of panicking.

## Notes

The second crate of the same project (`rsk-logic`, 7 harnesses) passes
with `-Z valid-value-checks`, `-Z uninit-checks`, and
`--extra-pointer-checks`; the crash needs a slice iterator in the harness's
reachable code. `-Z uninit-checks` on the affected crate reports two
harnesses as failed for a different, documented reason (memory
initialization of `[MaybeUninit<u32>; 12]` and
`UnsafeCell<Option<u32>>` is "not supported"), not a crash.
