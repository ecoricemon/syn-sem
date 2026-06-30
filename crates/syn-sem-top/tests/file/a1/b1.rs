// Fixture purpose: out-of-line child module loaded from `a1.rs`.
// For example, this file proves `mod b1;` maps to `~::a1::b1`.
// From a1.rs
// Logical path: ~::a1::b1
// Physical path: ~/a1/b1.rs

pub struct B1;

// Logical path: ~::a1::b1::b2
// Physical path: ~/a1/b1/b2.rs or ~/a1/b1/b2/mod.rs. We have the first one.
pub mod b2;
use b2::B2 as FromB2;
