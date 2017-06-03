extern crate glob;
use std::path::Path;

mod reader {
    pub mod class;
    pub mod runner;
}

pub fn run(filename: &Path) {
    let mut class_paths: Vec<String> = Vec::new();
    class_paths.push(String::from("/Users/sam/Personal/javart/java/lang/String.class"));
    class_paths.push(String::from("/Users/sam/Personal/javart/java/lang/System.class"));
    let class_result = reader::class::read(filename).unwrap();
    reader::runner::run(&class_paths, &class_result).unwrap();
}
