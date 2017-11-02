extern crate rjvm;
extern crate glob;
#[macro_use] extern crate assert_approx_eq;
extern crate checksum;

#[cfg(test)]
mod tests {
    use checksum::crc::Crc as crc;
    use rjvm::run_method;
    use rjvm::get_runtime;
    use rjvm::reader::runner::Variable;
    use rjvm::reader::runner::Runtime;
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::env;
    use std::fs;
    use std::process::Command;

    fn setup(source: &str) -> (Runtime, PathBuf) {
        let mut temp_dir = env::temp_dir();
        let mut source_path = PathBuf::new();
        source_path.push("tests/");
        source_path.push(source);
        source_path.set_extension("java");

        println!("src: {}", source_path.display());

        let crc = crc::new(source_path.to_str().unwrap()).checksum().unwrap().crc64;
        temp_dir.push(crc.to_string());

        println!("temp: {}", temp_dir.display());

        if !temp_dir.exists() {
            if fs::create_dir(temp_dir.as_path()).is_err() {
                panic!("Couldn't create temp dir {}", temp_dir.display());
            }

            let output = Command::new("javac")
                .args(&["-d", temp_dir.to_str().unwrap(), source_path.to_str().unwrap()])
                .output()
                .unwrap();
            assert!(output.status.success());
            println!("compiled: {}", temp_dir.display());
        }

        temp_dir.push(source);
        temp_dir.set_extension("class");

        return (get_runtime(&vec!(String::from(temp_dir.parent().unwrap().to_str().unwrap()))), temp_dir);
    }

    fn add_sub_mul_div_mod_test<F>(runtime: &mut Runtime, class_path: &Path, fn_name: &str, transform: F) where F: Fn(i32) -> Variable {
        let args = vec!(transform(11), transform(17), transform(3), transform(19), transform(5), transform(23));
        assert_eq!(run_method(runtime,
                              class_path,
                              fn_name,
                              &args,
                              transform(0).get_descriptor().as_str()
        ),
                   transform(-3));
    }

    fn shift_test<F>(runtime: &mut Runtime, class_path: &Path, fn_name: &str, transform: F, result: i64) where F: Fn(i64) -> Variable {
        let args = vec!(transform(-3), transform(4), transform(2), transform(2));
        assert_eq!(run_method(runtime,
                              class_path,
                              fn_name,
                              &args,
                              transform(0).get_descriptor().as_str()
        ),
                   transform(result));
    }

    #[test]
    fn get_component_type() {
        let (mut runtime, class_path) = setup("getComponentType");
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "getComponentTypeCheck1", &Vec::new(), "Ljava/lang/String;").extract_string(), "getComponentType$A");
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "getComponentTypeCheck2", &Vec::new(), "Ljava/lang/String;").extract_string(), "boolean");
    }

    #[test]
    fn maths() {
        let (mut runtime, class_path) = setup("maths");
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intAdd", 1, 2), 3);
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intAdd", 0x7FFFFFFF, 2), -0x7FFFFFFF);
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intSub", 123, 2), 121);
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intSub", -0x7FFFFFFF, 2), 0x7FFFFFFF);
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intMul", 0x10100100, 0x1001), 0x10200100);
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intDiv", 6, 3), 2);
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intDiv", <i32>::min_value(), -1), <i32>::min_value());
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intRem", 6, 4), 2);
        assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intRem", <i32>::min_value(), -1), 0);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longAdd", 0x123123123, 0x121212121), 0x244335244);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longAdd", 0x7FFFFFFFFFFFFFFF, 2), -0x7FFFFFFFFFFFFFFF);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longSub", 0x123123123, 0x123123120), 3);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longSub", -0x7FFFFFFFFFFFFFFF, 2), 0x7FFFFFFFFFFFFFFF);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longMul", 123, 100), 12300);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longMul", 0x1010010000000000, 0x1001), 0x1020010000000000);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longDiv", 1234, 2), 617);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longDiv", <i64>::min_value(), -1), <i64>::min_value());
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longRem", 1234, 3), 1);
        assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longRem", <i64>::min_value(), -1), 0);
        assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatAdd", 1.1, 2.2), 3.3);
        assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatSub", 4.1, 2.2), 1.9);
        assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatMul", 1.1, 2.0), 2.2);
        assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatDiv", 4.4, 1.1), 4.0);
        assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleAdd", 1.1, 2.2), 3.3);
        assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleSub", 4.1, 2.2), 1.9);
        assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleMul", 1.1, 2.0), 2.2);
        assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleDiv", 4.4, 1.1), 4.0);
        add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "shortAddSubMulDivMod", |x| Variable::Short(x as i16));
        add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "intAddSubMulDivMod", |x| Variable::Int(x as i32));
        add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "longAddSubMulDivMod", |x| Variable::Long(x as i64));
        add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "floatAddSubMulDivMod", |x| Variable::Float(x as f32));
        add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "doubleAddSubMulDivMod", |x| Variable::Double(x as f64));
        shift_test(&mut runtime, class_path.as_path(), "intShlShrUshr", |x| Variable::Int(x as i32), 0x3FFFFFFD as i64);
        shift_test(&mut runtime, class_path.as_path(), "longShlShrUshr", |x| Variable::Long(x as i64), 0x3FFFFFFFFFFFFFFD as i64);
    }

    #[test]
    fn lookupswitch() {
        let (mut runtime, class_path) = setup("lookupswitch");
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char('a')), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char('.')), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char('>')), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char(' ')), "Z"), Variable::Int(0));
    }

    #[test]
    fn tableswitch() {
        let (mut runtime, class_path) = setup("tableswitch");
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(1)), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(10)), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(11)), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(13)), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(15)), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(16)), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(130)), "Z"), Variable::Int(0));
    }

    #[test]
    fn string() {
        let (mut runtime, class_path) = setup("string");
        //assert_eq!(run_method(&mut runtime, class_path.as_path(), "getBytes", &Vec::new(), "B"), Variable::Byte('e' as u8));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "newAppendExtract", &Vec::new(), "C"), Variable::Int('a' as i32));
        assert_eq!(run_method(&mut runtime, class_path.as_path(), "copy", &Vec::new(), "C"), Variable::Int('o' as i32));
        assert_ne!(run_method(&mut runtime, class_path.as_path(), "getHashCode", &Vec::new(), "I"), Variable::Int(0));
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "intern"), 0x2);
    }

    #[test]
    fn try_catch() {
        let (mut runtime, class_path) = setup("trycatch");
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "test"), 0x2);
    }

    #[test]
    fn builtins_reflection() {
        let (mut runtime, class_path) = setup("builtins_reflection");
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "getCallerClassTest"), 0x1);
    }

    #[test]
    fn class_get_declared() {
        let (mut runtime, class_path) = setup("clazz");
        assert_eq!(void_bool_call(&mut runtime, class_path.as_path(), "checkSlots"), true);
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "getNumberOfFields"), 0x2);
        assert_eq!(void_str_call(&mut runtime, class_path.as_path(), "getNameOfFirstField"), "x");
    }

    fn void_bool_call(runtime: &mut Runtime, path: &Path, method: &str) -> bool {
        return run_method(runtime, path, method, &Vec::new(), "Z").to_bool();
    }

    fn void_int_call(runtime: &mut Runtime, path: &Path, method: &str) -> i32 {
        return run_method(runtime, path, method, &Vec::new(), "I").to_int();
    }

    fn void_str_call(runtime: &mut Runtime, path: &Path, method: &str) -> String {
        return run_method(runtime, path, method, &Vec::new(), "Ljava/lang/String;").extract_string();
    }

    fn int_int_call(runtime: &mut Runtime, path: &Path, method: &str, arg: i32) -> i32 {
        return run_method(runtime, path, method, &vec!(Variable::Int(arg)), "I").to_int();
    }

    fn int2_int_call(runtime: &mut Runtime, path: &Path, method: &str, arg: i32, arg2: i32) -> i32 {
        return run_method(runtime, path, method, &vec!(Variable::Int(arg), Variable::Int(arg2)), "I").to_int();
    }

    fn long2_long_call(runtime: &mut Runtime, path: &Path, method: &str, arg: i64, arg2: i64) -> i64 {
        return run_method(runtime, path, method, &vec!(Variable::Long(arg), Variable::Long(arg2)), "J").to_long();
    }

    fn float2_float_call(runtime: &mut Runtime, path: &Path, method: &str, arg: f32, arg2: f32) -> f32 {
        return run_method(runtime, path, method, &vec!(Variable::Float(arg), Variable::Float(arg2)), "F").to_float();
    }

    fn double2_double_call(runtime: &mut Runtime, path: &Path, method: &str, arg: f64, arg2: f64) -> f64 {
        return run_method(runtime, path, method, &vec!(Variable::Double(arg), Variable::Double(arg2)), "D").to_double();
    }

    #[test]
    fn arrays() {
        let (mut runtime, class_path) = setup("arrays");
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "arrayReturningFunctionTest"), 12);
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "arrayComparison"), 10);
    }

    #[test]
    fn inheritance() {
        let (mut runtime, class_path) = setup("inheritance");
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicImplementation"), 1);
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicImplementationExtension"), 2);
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicExtension"), 0x3987);
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicImplementationDowncast"), 3);
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "extendedMultipleImls"), 0x403);
    }

    #[test]
    fn multideps() {
        let (mut runtime, class_path) = setup("multideps");
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "test"), 6);
    }

    #[test]
    fn hash() {
        let (mut runtime, class_path) = setup("hash");
        let hashes = vec!(
                         int_int_call(&mut runtime, class_path.as_path(), "hashA", 1),
                         int_int_call(&mut runtime, class_path.as_path(), "hashA", 2),
                         int2_int_call(&mut runtime, class_path.as_path(), "hashB", 1, 0),
                         int2_int_call(&mut runtime, class_path.as_path(), "hashB", 1, 1),
                         int2_int_call(&mut runtime, class_path.as_path(), "hashB", 2, 0),
                         int2_int_call(&mut runtime, class_path.as_path(), "hashC", 0, 0),
                         int2_int_call(&mut runtime, class_path.as_path(), "hashC", 0, 1),
                         int2_int_call(&mut runtime, class_path.as_path(), "hashC", 1, 0),
                         int2_int_call(&mut runtime, class_path.as_path(), "circularHashD", 1, 0),
                         int2_int_call(&mut runtime, class_path.as_path(), "circularHashD", 2, 0),
                         int2_int_call(&mut runtime, class_path.as_path(), "circularHashE", 1, 0),
                         int2_int_call(&mut runtime, class_path.as_path(), "circularHashE", 2, 0),
                         );
        let mut set = HashSet::new();
        for hash in hashes {
            println!("Inserting hash {}", hash);
            assert_eq!(set.contains(&hash), false);
            set.insert(hash);
        }
    }

    #[test]
    fn static_init() {
        let (mut runtime, class_path) = setup("static_init");
        assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "getx"), 1);
    }
}