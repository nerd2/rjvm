extern crate glob;
use std::path::Path;

pub mod reader {
    #[macro_use]
    pub mod class_reader;
    #[macro_use]
    pub mod runner;
    mod util;
    mod builtins;
    mod types {
        pub mod class;
        pub mod constant_pool;
        pub mod frame;
        pub mod objects;
        pub mod runtime;
        pub mod variable;
    }
}

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
    return reader::runner::get_runtime(&my_class_paths);
}

pub fn run_method(runtime: &mut reader::runner::Runtime, filename: &Path, method: &str, arguments: &Vec<reader::runner::Variable>, return_descriptor: &str) -> reader::runner::Variable {
    let class_result = reader::class_reader::read(filename).unwrap();
    return reader::runner::run_method(runtime, &class_result, method, arguments, String::from(return_descriptor)).unwrap();
}
