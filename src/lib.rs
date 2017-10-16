extern crate glob;
use std::path::Path;

pub mod reader {
    pub mod class;
    pub mod runner;
}

fn get_class_paths() -> Vec<String> {
    return vec!(String::from("./rt/"), String::from("./javart/"));
}

pub fn run(filename: &Path) {
    let class_result = reader::class::read(filename).unwrap();
    reader::runner::run(&get_class_paths(), &class_result).unwrap();
}

pub fn run_method(filename: &Path, method: &str, arguments: &Vec<reader::runner::Variable>, return_type: Option<&reader::runner::Variable>, extra_class_paths: &Vec<String>) -> reader::runner::Variable {
    let mut my_class_paths = get_class_paths();
    for p in extra_class_paths {
        my_class_paths.insert(0, p.clone());
    }
    println!("My class paths {:?}", my_class_paths);
    let class_result = reader::class::read(filename).unwrap();
    return reader::runner::run_method(&my_class_paths, &class_result, method, arguments, return_type).unwrap();
}
