use crate::{
    console::{console_read, console_write},
    param::NDEV,
};

pub static DEV_SW: [Option<DevSW>; NDEV] = {
    let mut sw = [None; NDEV];

    // connect read and write system calls
    // to the console_read and console_write functions.
    sw[CONSOLE] = Some(DevSW {
        read: console_read,
        write: console_write,
    });
    sw
};

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
