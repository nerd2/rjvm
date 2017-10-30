extern crate rjvm;
extern crate glob;
#[macro_use] extern crate assert_approx_eq;

#[cfg(test)]
mod tests {
    use rjvm::run_method;
    use rjvm::get_runtime;
    use rjvm::reader::runner::Variable;
    use rjvm::reader::runner::Runtime;
    use std::collections::HashSet;
    use std::path::Path;

    fn add_sub_mul_div_mod_test<F>(runtime: &mut Runtime, fn_name: &str, transform: F) where F: Fn(i32) -> Variable {
        let args = vec!(transform(11), transform(17), transform(3), transform(19), transform(5), transform(23));
        assert_eq!(run_method(runtime,
            Path::new("tests/maths.class"),
            fn_name,
            &args,
            transform(0).get_descriptor().as_str()
        ),
        transform(-3));
    }

    fn shift_test<F>(runtime: &mut Runtime, fn_name: &str, transform: F, result: i64) where F: Fn(i64) -> Variable {
        let args = vec!(transform(-3), transform(4), transform(2), transform(2));
        assert_eq!(run_method(runtime,
            Path::new("tests/maths.class"),
            fn_name,
            &args,
            transform(0).get_descriptor().as_str()
        ),
        transform(result));
    }

    #[test]
    fn get_component_type() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(run_method(&mut runtime, Path::new("tests/getComponentType.class"), "getComponentTypeCheck1", &Vec::new(), "Ljava/lang/String;").extract_string(), "getComponentType$A");
        assert_eq!(run_method(&mut runtime, Path::new("tests/getComponentType.class"), "getComponentTypeCheck2", &Vec::new(), "Ljava/lang/String;").extract_string(), "boolean");
    }

    #[test]
    fn maths() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intAdd", 1, 2), 3);
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intAdd", 0x7FFFFFFF, 2), -0x7FFFFFFF);
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intSub", 123, 2), 121);
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intSub", -0x7FFFFFFF, 2), 0x7FFFFFFF);
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intMul", 0x10100100, 0x1001), 0x10200100);
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intDiv", 6, 3), 2);
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intDiv", <i32>::min_value(), -1), <i32>::min_value());
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intRem", 6, 4), 2);
        assert_eq!(int2_int_call(&mut runtime, "tests/maths.class", "intRem", <i32>::min_value(), -1), 0);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longAdd", 0x123123123, 0x121212121), 0x244335244);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longAdd", 0x7FFFFFFFFFFFFFFF, 2), -0x7FFFFFFFFFFFFFFF);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longSub", 0x123123123, 0x123123120), 3);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longSub", -0x7FFFFFFFFFFFFFFF, 2), 0x7FFFFFFFFFFFFFFF);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longMul", 123, 100), 12300);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longMul", 0x1010010000000000, 0x1001), 0x1020010000000000);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longDiv", 1234, 2), 617);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longDiv", <i64>::min_value(), -1), <i64>::min_value());
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longRem", 1234, 3), 1);
        assert_eq!(long2_long_call(&mut runtime, "tests/maths.class", "longRem", <i64>::min_value(), -1), 0);
        assert_approx_eq!(float2_float_call(&mut runtime, "tests/maths.class", "floatAdd", 1.1, 2.2), 3.3);
        assert_approx_eq!(float2_float_call(&mut runtime, "tests/maths.class", "floatSub", 4.1, 2.2), 1.9);
        assert_approx_eq!(float2_float_call(&mut runtime, "tests/maths.class", "floatMul", 1.1, 2.0), 2.2);
        assert_approx_eq!(float2_float_call(&mut runtime, "tests/maths.class", "floatDiv", 4.4, 1.1), 4.0);
        assert_approx_eq!(double2_double_call(&mut runtime, "tests/maths.class", "doubleAdd", 1.1, 2.2), 3.3);
        assert_approx_eq!(double2_double_call(&mut runtime, "tests/maths.class", "doubleSub", 4.1, 2.2), 1.9);
        assert_approx_eq!(double2_double_call(&mut runtime, "tests/maths.class", "doubleMul", 1.1, 2.0), 2.2);
        assert_approx_eq!(double2_double_call(&mut runtime, "tests/maths.class", "doubleDiv", 4.4, 1.1), 4.0);
        add_sub_mul_div_mod_test(&mut runtime, "shortAddSubMulDivMod", |x| Variable::Short(x as i16));
        add_sub_mul_div_mod_test(&mut runtime, "intAddSubMulDivMod", |x| Variable::Int(x as i32));
        add_sub_mul_div_mod_test(&mut runtime, "longAddSubMulDivMod", |x| Variable::Long(x as i64));
        add_sub_mul_div_mod_test(&mut runtime, "floatAddSubMulDivMod", |x| Variable::Float(x as f32));
        add_sub_mul_div_mod_test(&mut runtime, "doubleAddSubMulDivMod", |x| Variable::Double(x as f64));
        shift_test(&mut runtime, "intShlShrUshr", |x| Variable::Int(x as i32), 0x3FFFFFFD as i64);
        shift_test(&mut runtime, "longShlShrUshr", |x| Variable::Long(x as i64), 0x3FFFFFFFFFFFFFFD as i64);
    }

    #[test]
    fn lookupswitch() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(run_method(&mut runtime, Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char('a')), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char('.')), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char('>')), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char(' ')), "Z"), Variable::Int(0));
    }

    #[test]
    fn tableswitch() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(run_method(&mut runtime, Path::new("tests/tableswitch.class"), "check", &vec!(Variable::Int(1)), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, Path::new("tests/tableswitch.class"), "check", &vec!(Variable::Int(10)), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, Path::new("tests/tableswitch.class"), "check", &vec!(Variable::Int(11)), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, Path::new("tests/tableswitch.class"), "check", &vec!(Variable::Int(13)), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, Path::new("tests/tableswitch.class"), "check", &vec!(Variable::Int(15)), "Z"), Variable::Int(1));
        assert_eq!(run_method(&mut runtime, Path::new("tests/tableswitch.class"), "check", &vec!(Variable::Int(16)), "Z"), Variable::Int(0));
        assert_eq!(run_method(&mut runtime, Path::new("tests/tableswitch.class"), "check", &vec!(Variable::Int(130)), "Z"), Variable::Int(0));
    }

    #[test]
    fn string_basics() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        //assert_eq!(run_method(&mut runtime, Path::new("tests/string.class"), "getBytes", &Vec::new(), "B"), Variable::Byte('e' as u8));
        assert_eq!(run_method(&mut runtime, Path::new("tests/string.class"), "newAppendExtract", &Vec::new(), "C"), Variable::Int('a' as i32));
        assert_eq!(run_method(&mut runtime, Path::new("tests/string.class"), "copy", &Vec::new(), "C"), Variable::Int('o' as i32));
        assert_ne!(run_method(&mut runtime, Path::new("tests/string.class"), "getHashCode", &Vec::new(), "I"), Variable::Int(0));
    }

    #[test]
    fn string_intern() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(void_int_call(&mut runtime, "tests/string.class", "intern"), 0x2);
    }

    #[test]
    fn builtins_reflection() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(void_int_call(&mut runtime, "tests/builtins_reflection.class", "getCallerClassTest"), 0x1);
    }

    #[test]
    #[ignore]
    fn class_get_declared() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(void_int_call(&mut runtime, "tests/clazz.class", "getNumberOfFields"), 0x2);
    }

    fn void_int_call(runtime: &mut Runtime, path: &str, method: &str) -> i32 {
        return run_method(runtime, Path::new(path), method, &Vec::new(), "I").to_int();
    }

    fn int_int_call(runtime: &mut Runtime, path: &str, method: &str, arg: i32) -> i32 {
        return run_method(runtime, Path::new(path), method, &vec!(Variable::Int(arg)), "I").to_int();
    }

    fn int2_int_call(runtime: &mut Runtime, path: &str, method: &str, arg: i32, arg2: i32) -> i32 {
        return run_method(runtime, Path::new(path), method, &vec!(Variable::Int(arg), Variable::Int(arg2)), "I").to_int();
    }

    fn long2_long_call(runtime: &mut Runtime, path: &str, method: &str, arg: i64, arg2: i64) -> i64 {
        return run_method(runtime, Path::new(path), method, &vec!(Variable::Long(arg), Variable::Long(arg2)), "J").to_long();
    }

    fn float2_float_call(runtime: &mut Runtime, path: &str, method: &str, arg: f32, arg2: f32) -> f32 {
        return run_method(runtime, Path::new(path), method, &vec!(Variable::Float(arg), Variable::Float(arg2)), "F").to_float();
    }

    fn double2_double_call(runtime: &mut Runtime, path: &str, method: &str, arg: f64, arg2: f64) -> f64 {
        return run_method(runtime, Path::new(path), method, &vec!(Variable::Double(arg), Variable::Double(arg2)), "D").to_double();
    }

    #[test]
    fn arrays() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(void_int_call(&mut runtime, "tests/arrays.class", "arrayReturningFunctionTest"), 12);
        assert_eq!(void_int_call(&mut runtime, "tests/arrays.class", "arrayComparison"), 10);
    }

    #[test]
    fn inheritance() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(void_int_call(&mut runtime, "tests/inheritance.class", "basicImplementation"), 1);
        assert_eq!(void_int_call(&mut runtime, "tests/inheritance.class", "basicImplementationExtension"), 2);
        assert_eq!(void_int_call(&mut runtime, "tests/inheritance.class", "basicExtension"), 0x3987);
        assert_eq!(void_int_call(&mut runtime, "tests/inheritance.class", "basicImplementationDowncast"), 3);
        assert_eq!(void_int_call(&mut runtime, "tests/inheritance.class", "extendedMultipleImls"), 0x403);
    }

    #[test]
    fn multideps() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(void_int_call(&mut runtime, "tests/multideps.class", "test"), 6);
    }

    #[test]
    fn hash() {
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        let hashes = vec!(
                         int_int_call(&mut runtime, "tests/hash.class", "hashA", 1),
                         int_int_call(&mut runtime, "tests/hash.class", "hashA", 2),
                         int2_int_call(&mut runtime, "tests/hash.class", "hashB", 1, 0),
                         int2_int_call(&mut runtime, "tests/hash.class", "hashB", 1, 1),
                         int2_int_call(&mut runtime, "tests/hash.class", "hashB", 2, 0),
                         int2_int_call(&mut runtime, "tests/hash.class", "hashC", 0, 0),
                         int2_int_call(&mut runtime, "tests/hash.class", "hashC", 0, 1),
                         int2_int_call(&mut runtime, "tests/hash.class", "hashC", 1, 0),
                         int2_int_call(&mut runtime, "tests/hash.class", "circularHashD", 1, 0),
                         int2_int_call(&mut runtime, "tests/hash.class", "circularHashD", 2, 0),
                         int2_int_call(&mut runtime, "tests/hash.class", "circularHashE", 1, 0),
                         int2_int_call(&mut runtime, "tests/hash.class", "circularHashE", 2, 0),
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
        let mut runtime = get_runtime(&vec!(String::from("./tests/")));
        assert_eq!(void_int_call(&mut runtime, "tests/static_init.class", "getx"), 1);
    }
}