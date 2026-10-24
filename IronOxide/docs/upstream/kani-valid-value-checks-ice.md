# Kani: internal compiler error with `-Z valid-value-checks` on slice iteration

Prepared 2026-10-23 for https://github.com/model-checking/kani/issues (the
development machine has no GitHub CLI; file by hand from this text). Kept
here as the record behind Verification_Plan section 6.1, which leaves the
invalid-value checks off for `rsk-kernel` until this is fixed.

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
