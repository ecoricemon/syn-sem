// From a1.rs
// Logical path: ~::a1::b1
// Physical path: ~/a1/b1.rs

pub struct B1;

// Logical path: ~::a1::b1::b2
// Physical path: ~/a1/b1/b2.rs or ~/a1/b1/b2/mod.rs. We have the first one.
pub mod b2;
use b2::B2 as FromB2;
