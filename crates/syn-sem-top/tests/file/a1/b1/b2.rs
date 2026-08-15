// Fixture purpose: nested out-of-line child module loaded from `a1/b1.rs`.
// For example, this file proves `mod b2;` maps to `~::a1::b1::b2`.
// From a1/b1.rs
// Logical path: ~::a1::b1::b2
// Physical path: ~/a1/b1/b2.rs

pub const DEPTH: usize = 7;

pub struct B2;
