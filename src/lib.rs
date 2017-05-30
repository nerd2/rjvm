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
        let maybeClass = reader::class::read(&file);

        if maybeClass.is_ok() {
            let class = maybeClass.unwrap();
            let className = String::from(get_cp_class_name(&class.constant_pool, class.this_class_index).unwrap());
            println!("Loaded class {}", className);
            jci_classes.insert(className, class);
        } else {
            println!("Failed to load class");
        }
    }
    let class_result = reader::class::read(filename).unwrap();
    reader::runner::run(&jci_classes, &class_result).unwrap();
}
