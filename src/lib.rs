extern crate glob;
use std::path::Path;

pub mod reader {
    pub mod class;
    pub mod runner;
}

fn get_class_paths() -> Vec<String> {
    return vec!(String::from("/Users/sam/Personal/rjvm/rt/"), String::from("/Users/sam/Personal/javart/"));
}

pub fn run(filename: &Path) {
    let class_paths = get_class_paths();
    let class_result = reader::class::read(filename).unwrap();
    reader::runner::run(&class_paths, &class_result).unwrap();
}

pub fn run_method(filename: &Path, method: &str, arguments: &Vec<reader::runner::Variable>, return_type: Option<&reader::runner::Variable>) -> reader::runner::Variable {
    let class_paths = get_class_paths();
    let class_result = reader::class::read(filename).unwrap();
    return reader::runner::run_method(&class_paths, &class_result, method, arguments, return_type).unwrap();
}
