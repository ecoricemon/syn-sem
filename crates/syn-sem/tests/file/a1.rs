// Entry file
// Logical path: ~::a1
// Physical path: ~/a1

// Logical path: ~::a1::b1
// Physical path: ~/a1/b1.rs or ~/a1/b1/mod.rs. We have the first one.
mod b1;

// Logical path: ~::a1::c1
// Physical path: ~/c1.rs
#[path = "c1.rs"]
mod c1;

// Logical path: ~::a1::dx
// Physical path: ~/d1
#[path = "d1"]
mod dx {
    // Logical path: ~::a1::dx::d2
    // Physical path: ~/d1/d2.rs or ~d1/d2/mod.rs. We have the first one.
    mod d2;
}

// Logical path: ~::a1::e1
// Physical path: ~/a1/e1
mod e1 {
    // Logical path: ~::a1::e1::e2
    // Physical path: ~/a1/e1/e2.rs or ~/a1/e1/e2/mod.rs. We have the first one.
    mod e2;

    // Logical path: ~::a1::e1::e3
    // Physical path: ~/a1/e1/e4.rs
    #[path = "e4.rs"]
    mod e3;
}
