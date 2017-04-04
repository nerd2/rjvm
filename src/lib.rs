use std::path::Path;

mod reader {
    pub mod class;
}

pub fn run(filename: &Path) {
    reader::class::read(filename);
}
