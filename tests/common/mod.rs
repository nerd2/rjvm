#![allow(dead_code)]

extern crate rjvm;

pub use self::rjvm::get_runtime;
pub use self::rjvm::run_method;
pub use self::rjvm::Variable;
pub use self::rjvm::Runtime;
pub use self::rjvm::make_string;
pub use std::path::{Path, PathBuf};

use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::Hash;
use std::hash::Hasher;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub fn setup(classname: &str, source_body: &str) -> (Runtime, PathBuf) {
    let source = String::from(source_body);

    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let crc = hasher.finish();
    let mut temp_dir = env::temp_dir();
    temp_dir.push(crc.to_string());

    println!("temp: {}", temp_dir.display());

    if !temp_dir.exists() {
        if fs::create_dir(temp_dir.as_path()).is_err() {
            panic!("Couldn't create temp dir {}", temp_dir.display());
        }

        let mut source_path = temp_dir.clone();
        source_path.push(classname);
        source_path.set_extension("java");
        let mut file = File::create(source_path.as_path()).expect("Couldn't open source file for writing");
        file.write_all(source.as_bytes()).expect("Unable to write source file");

        let output = Command::new("javac")
            .args(&["-d", temp_dir.to_str().unwrap(), source_path.to_str().unwrap()])
            .output()
            .unwrap();
        if output.status.success() == false {
            fs::remove_dir_all(temp_dir).expect("Failed to remove temp dir after compilation failure");
            panic!("failed to compile: {}", String::from_utf8_lossy(&output.stderr));
        }
        println!("compiled: {}", temp_dir.display());
    }

    temp_dir.push(classname);
    temp_dir.set_extension("class");

    return (get_runtime(&vec!(String::from(temp_dir.parent().unwrap().to_str().unwrap()))), temp_dir);
}


pub fn void_bool_call(runtime: &mut Runtime, path: &Path, method: &str) -> bool {
    return run_method(runtime, path, method, &Vec::new(), "Z").to_bool();
}

pub fn void_int_call(runtime: &mut Runtime, path: &Path, method: &str) -> i32 {
    return run_method(runtime, path, method, &Vec::new(), "I").to_int();
}

pub fn void_str_call(runtime: &mut Runtime, path: &Path, method: &str) -> String {
    return run_method(runtime, path, method, &Vec::new(), "Ljava/lang/String;").extract_string();
}

pub fn int_int_call(runtime: &mut Runtime, path: &Path, method: &str, arg: i32) -> i32 {
    return run_method(runtime, path, method, &vec!(Variable::Int(arg)), "I").to_int();
}

pub fn int2_int_call(runtime: &mut Runtime, path: &Path, method: &str, arg: i32, arg2: i32) -> i32 {
    return run_method(runtime, path, method, &vec!(Variable::Int(arg), Variable::Int(arg2)), "I").to_int();
}

pub fn long2_long_call(runtime: &mut Runtime, path: &Path, method: &str, arg: i64, arg2: i64) -> i64 {
    return run_method(runtime, path, method, &vec!(Variable::Long(arg), Variable::Long(arg2)), "J").to_long();
}

pub fn float2_float_call(runtime: &mut Runtime, path: &Path, method: &str, arg: f32, arg2: f32) -> f32 {
    return run_method(runtime, path, method, &vec!(Variable::Float(arg), Variable::Float(arg2)), "F").to_float();
}

pub fn double2_double_call(runtime: &mut Runtime, path: &Path, method: &str, arg: f64, arg2: f64) -> f64 {
    return run_method(runtime, path, method, &vec!(Variable::Double(arg), Variable::Double(arg2)), "D").to_double();
}

pub fn str_str_call(runtime: &mut Runtime, path: &Path, method: &str, arg: &str) -> Option<String> {
    let argvar = make_string(runtime, arg).expect("Couldn't create string for argument");
    let ret = run_method(runtime, path, method, &vec!(argvar), "Ljava/lang/String;");
    if ret.is_null() {
        return None;
    } else {
        return Some(ret.extract_string());
    }
}

pub fn str2_void_call(runtime: &mut Runtime, path: &Path, method: &str, arg1: &str, arg2: &str) {
    let argvar1 = make_string(runtime, arg1).expect("Couldn't create string for argument");
    let argvar2 = make_string(runtime, arg2).expect("Couldn't create string for argument");
    run_method(runtime, path, method, &vec!(argvar1, argvar2), "V");
}

