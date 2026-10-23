//@ scope: application
//@ R3.4: attributes that lower a Profile_Lint lint below error fail; other lints may be allowed.
#![allow(clippy::disallowed_methods)]
#![allow(dead_code)]

#[allow(clippy::all)]
fn a() {}

#[cfg_attr(test, allow(clippy::disallowed_macros))]
fn b() {}

#[expect(warnings)]
fn c() {}

#[allow(unused_variables, clippy::needless_return)]
fn d() {}

#[warn(clippy::disallowed_types)]
fn e() {}
