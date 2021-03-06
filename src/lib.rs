#![deny(
non_snake_case,
unreachable_code,
unused_assignments,
unused_imports,
unused_must_use,
unused_variables,
unused_mut,
)]

extern crate glob;
extern crate os_type;
extern crate zip;
#[macro_use]
extern crate lazy_static;
use std::path::Path;
use std::process::Command;

mod reader;

use std::fs::File;
use std::io::BufReader;

pub use reader::runner::Runtime;
pub use reader::runner::Variable;
pub use reader::runner::make_string;

fn get_rt_jar() -> Vec<zip::ZipArchive<File>> {
    let rt_path : String = match os_type::current_platform().os_type {
        os_type::OSType::OSX => {
            let jdk_path = Command::new("/usr/libexec/java_home").arg("-v").arg("1.8").output().expect("Failed to determine JDK location");
            let jdk_path_str = String::from_utf8_lossy(&jdk_path.stdout);
            jdk_path_str.replace('\n',"") + "/jre/lib/rt.jar"
        }
        os_type::OSType::Ubuntu | os_type::OSType::Debian => {
            String::from("/usr/lib/jvm/java-8-openjdk-amd64/jre/lib/rt.jar")
        }
        _ => {
            panic!("Unsupported system");
        }
    };

    let path = Path::new(&rt_path);
    let maybe_file = File::open(path);
    if maybe_file.is_err() {
        panic!("Couldn't open rt jar file {}", maybe_file.unwrap_err());
    }

    let maybe_zip = zip::ZipArchive::new(maybe_file.unwrap());

    if maybe_zip.is_err() {
        panic!("Couldn't load rt zip {:?}", maybe_zip.unwrap_err());
    }

    return vec!(maybe_zip.unwrap());
}

fn read(filename: &Path) -> reader::class_reader::ClassResult {
    let reader = File::open(filename).expect(format!("Could not open {}", filename.display()).as_str());
    let mut buf_reader = BufReader::new(reader);
    let mut class_result = reader::class_reader::read_stage_1(&mut buf_reader).expect("Couldn't read headers of class file");
    reader::class_reader::read_stage_2(&mut buf_reader, &mut class_result).expect("Couldn't read rest of class file");
    return class_result;
}


pub fn get_runtime(class_paths: &Vec<String>) -> reader::runner::Runtime {
    return reader::runner::get_runtime(class_paths, get_rt_jar(), true);
}

pub fn get_runtime_bypass_initialisation(class_paths: &Vec<String>) -> Runtime {
    return reader::runner::get_runtime(class_paths, get_rt_jar(), false);
}

pub fn run_method(runtime: &mut reader::runner::Runtime, filename: &Path, method: &str, arguments: &Vec<reader::runner::Variable>, return_descriptor: &str) -> reader::runner::Variable {
    let class_result = read(filename);
    return reader::runner::run_method(runtime, &class_result, method, arguments, String::from(return_descriptor)).unwrap();
}
