use crate::param::NDEV;

pub static DEV_SW: [Option<DevSW>; NDEV] = [None; NDEV];

pub const CONSOLE: usize = 1;

pub struct File {
    r#type: Type,
}

enum Type {
    None,
    Pipe,
    Inode,
    Device,
}

/// In-memory copy of an inode.
pub struct Inode {
    dev: usize,   // Device number.
    inum: usize,  // Inode number.
    r#ref: usize, // Reference count.
}

#[derive(Clone, Copy)]
pub struct DevSW {
    read: fn(i32, u64, i32) -> i32,
    write: fn(i32, u64, i32) -> i32,
}
