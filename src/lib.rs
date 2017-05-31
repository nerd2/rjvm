extern crate glob;
use std::path::{Path, PathBuf};
use glob::glob;
use std::collections::HashMap;
use reader::class::*;

mod reader {
    pub mod class;
    pub mod runner;
}

pub fn run(filename: &Path) {
    let mut jci_classes: HashMap<String, ClassResult> = HashMap::new();
    for file in glob("jcl/**/*.class").unwrap().filter_map(Result::ok) {
        let maybe_class = reader::class::read(&file);

        if maybe_class.is_ok() {
            let class = maybe_class.unwrap();
            let class_name = String::from(get_cp_class_name(&class.constant_pool, class.this_class_index).unwrap());
            println!("Loaded class {}", class_name);
            jci_classes.insert(class_name, class);
        } else {
            println!("Failed to load class");
        }
    }
    let class_result = reader::class::read(filename).unwrap();
    reader::runner::run(&jci_classes, &class_result).unwrap();
}
