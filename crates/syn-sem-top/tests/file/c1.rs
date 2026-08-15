// Fixture purpose: sibling file loaded by a `#[path]` module declaration.
// For example, this file proves `#[path = "c1.rs"] mod c1;` maps to `~::a1::c1`.
// From a1.rs
// Logical path: ~::a1::c1
// Physical path: ~/c1.rs

pub struct C1;

pub const CAPACITY: usize = 2 + 3;
