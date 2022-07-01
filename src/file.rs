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
