#![deny(
non_snake_case,
unreachable_code,
unused_assignments,
unused_imports,
unused_variables,
unused_mut,
)]

extern crate glob;
use std::path::Path;

mod reader;

pub use reader::runner::Runtime;
pub use reader::runner::Variable;
pub use reader::runner::make_string;

fn get_class_paths() -> Vec<String> {
    return vec!(String::from("./javart/"));
}

pub fn run(filename: &Path) {
    let class_result = reader::class_reader::read(filename).unwrap();
    reader::runner::run(&get_class_paths(), &class_result).unwrap();
}

pub fn get_runtime(class_paths: &Vec<String>) -> reader::runner::Runtime {
    let mut my_class_paths = get_class_paths();
    for p in class_paths {
        my_class_paths.insert(0, p.clone());
    }
    println!("My class paths {:?}", my_class_paths);
    return reader::runner::get_runtime(&my_class_paths, true);
}

pub fn get_runtime_bypass_initialisation(class_paths: &Vec<String>) -> Runtime {
    let mut my_class_paths = get_class_paths();
    for p in class_paths {
        my_class_paths.insert(0, p.clone());
    }
    println!("My class paths {:?}", my_class_paths);
    return reader::runner::get_runtime(&my_class_paths, false);
}

pub fn run_method(runtime: &mut reader::runner::Runtime, filename: &Path, method: &str, arguments: &Vec<reader::runner::Variable>, return_descriptor: &str) -> reader::runner::Variable {
    let class_result = reader::class_reader::read(filename).unwrap();
    return reader::runner::run_method(runtime, &class_result, method, arguments, String::from(return_descriptor)).unwrap();
}
