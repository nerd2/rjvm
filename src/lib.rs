extern crate glob;
use std::path::Path;

pub mod reader {
    pub mod class;
    pub mod runner;
}

pub fn run(filename: &Path) {
    let class_paths = vec!(String::from("/Users/sam/Personal/javart/"));
    let class_result = reader::class::read(filename).unwrap();
    reader::runner::run(&class_paths, &class_result).unwrap();
}

pub fn run_method(filename: &Path, method: &str, arguments: &Vec<reader::runner::Variable>, return_type: Option<&reader::runner::Variable>) -> reader::runner::Variable {
    let class_paths = vec!(String::from("/Users/sam/Personal/javart/"));
    let class_result = reader::class::read(filename).unwrap();
    return reader::runner::run_method(&class_paths, &class_result, method, arguments, return_type).unwrap();
}
