//! Uses an unstable language feature. In a workspace crate even a gate that
//! the stable build never enables (`docsrs`) is rejected.
#![forbid(unsafe_code)]
#![feature(never_type)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub fn never() -> Option<!> {
    None
}
