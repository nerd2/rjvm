extern crate rjvm;
extern crate glob;
#[macro_use] extern crate assert_approx_eq;

#[cfg(test)]
mod tests {
    use rjvm::run_method;
    use rjvm::reader::runner::Variable;
    use std::collections::HashSet;
    use std::path::Path;

    fn add_sub_mul_div_mod_test<F>(fn_name: &str, transform: F) where F: Fn(i32) -> Variable {
        let args = vec!(transform(11), transform(17), transform(3), transform(19), transform(5), transform(23));
        assert_eq!(run_method(
            Path::new("tests/maths.class"),
            fn_name,
            &args,
            transform(0).get_descriptor().as_str(),
            &Vec::new()
        ),
        transform(-3));
    }

    fn shift_test<F>(fn_name: &str, transform: F, result: i64) where F: Fn(i64) -> Variable {
        let args = vec!(transform(-3), transform(4), transform(2), transform(2));
        assert_eq!(run_method(
            Path::new("tests/maths.class"),
            fn_name,
            &args,
            transform(0).get_descriptor().as_str(),
            &Vec::new()),
        transform(result));
    }

    #[test]
    fn get_component_type() {
        assert_eq!(run_method(Path::new("tests/getComponentType.class"), "getComponentTypeCheck1", &Vec::new(), "Ljava/lang/String;", &vec!(String::from("./tests/"))).extract_string(), "LgetComponentType$A;");
        assert_eq!(run_method(Path::new("tests/getComponentType.class"), "getComponentTypeCheck2", &Vec::new(), "Ljava/lang/String;", &vec!(String::from("./tests/"))).extract_string(), "Z");
    }

    #[test]
    fn maths() {
        assert_eq!(int2_int_call("tests/maths.class", "intAdd", 1, 2), 3);
        assert_eq!(int2_int_call("tests/maths.class", "intAdd", 0x7FFFFFFF, 2), -0x7FFFFFFF);
        assert_eq!(int2_int_call("tests/maths.class", "intSub", 123, 2), 121);
        assert_eq!(int2_int_call("tests/maths.class", "intSub", -0x7FFFFFFF, 2), 0x7FFFFFFF);
        assert_eq!(int2_int_call("tests/maths.class", "intMul", 0x10100100, 0x1001), 0x10200100);
        assert_eq!(int2_int_call("tests/maths.class", "intDiv", 6, 3), 2);
        assert_eq!(int2_int_call("tests/maths.class", "intDiv", <i32>::min_value(), -1), <i32>::min_value());
        assert_eq!(int2_int_call("tests/maths.class", "intRem", 6, 4), 2);
        assert_eq!(int2_int_call("tests/maths.class", "intRem", <i32>::min_value(), -1), 0);
        assert_eq!(long2_long_call("tests/maths.class", "longAdd", 0x123123123, 0x121212121), 0x244335244);
        assert_eq!(long2_long_call("tests/maths.class", "longAdd", 0x7FFFFFFFFFFFFFFF, 2), -0x7FFFFFFFFFFFFFFF);
        assert_eq!(long2_long_call("tests/maths.class", "longSub", 0x123123123, 0x123123120), 3);
        assert_eq!(long2_long_call("tests/maths.class", "longSub", -0x7FFFFFFFFFFFFFFF, 2), 0x7FFFFFFFFFFFFFFF);
        assert_eq!(long2_long_call("tests/maths.class", "longMul", 123, 100), 12300);
        assert_eq!(long2_long_call("tests/maths.class", "longMul", 0x1010010000000000, 0x1001), 0x1020010000000000);
        assert_eq!(long2_long_call("tests/maths.class", "longDiv", 1234, 2), 617);
        assert_eq!(long2_long_call("tests/maths.class", "longDiv", <i64>::min_value(), -1), <i64>::min_value());
        assert_eq!(long2_long_call("tests/maths.class", "longRem", 1234, 3), 1);
        assert_eq!(long2_long_call("tests/maths.class", "longRem", <i64>::min_value(), -1), 0);
        assert_approx_eq!(float2_float_call("tests/maths.class", "floatAdd", 1.1, 2.2), 3.3);
        assert_approx_eq!(float2_float_call("tests/maths.class", "floatSub", 4.1, 2.2), 1.9);
        assert_approx_eq!(float2_float_call("tests/maths.class", "floatMul", 1.1, 2.0), 2.2);
        assert_approx_eq!(float2_float_call("tests/maths.class", "floatDiv", 4.4, 1.1), 4.0);
        assert_approx_eq!(double2_double_call("tests/maths.class", "doubleAdd", 1.1, 2.2), 3.3);
        assert_approx_eq!(double2_double_call("tests/maths.class", "doubleSub", 4.1, 2.2), 1.9);
        assert_approx_eq!(double2_double_call("tests/maths.class", "doubleMul", 1.1, 2.0), 2.2);
        assert_approx_eq!(double2_double_call("tests/maths.class", "doubleDiv", 4.4, 1.1), 4.0);
        add_sub_mul_div_mod_test("shortAddSubMulDivMod", |x| Variable::Short(x as i16));
        add_sub_mul_div_mod_test("intAddSubMulDivMod", |x| Variable::Int(x as i32));
        add_sub_mul_div_mod_test("longAddSubMulDivMod", |x| Variable::Long(x as i64));
        add_sub_mul_div_mod_test("floatAddSubMulDivMod", |x| Variable::Float(x as f32));
        add_sub_mul_div_mod_test("doubleAddSubMulDivMod", |x| Variable::Double(x as f64));
        shift_test("intShlShrUshr", |x| Variable::Int(x as i32), 0x3FFFFFFD as i64);
        shift_test("longShlShrUshr", |x| Variable::Long(x as i64), 0x3FFFFFFFFFFFFFFD as i64);
    }

    #[test]
    fn lookupswitch() {
        assert_eq!(run_method(Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char('a')), "Z", &Vec::new()), Variable::Int(0));
        assert_eq!(run_method(Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char('.')), "Z", &Vec::new()), Variable::Int(1));
        assert_eq!(run_method(Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char('>')), "Z", &Vec::new()), Variable::Int(1));
        assert_eq!(run_method(Path::new("tests/lookupswitch.class"), "check", &vec!(Variable::Char(' ')), "Z", &Vec::new()), Variable::Int(0));
    }

    #[test]
    fn string_basics() {
        //assert_eq!(run_method(Path::new("tests/string.class"), "newAppendExtract", &Vec::new(), "C", &Vec::new()), Variable::Int('a' as i32));
        assert_eq!(run_method(Path::new("tests/string.class"), "copy", &Vec::new(), "C", &Vec::new()), Variable::Int('o' as i32));
        //assert_eq!(run_method(Path::new("tests/string.class"), "getBytes", &Vec::new(), "B", &Vec::new()), Variable::Byte('e' as u8));
        assert_ne!(run_method(Path::new("tests/string.class"), "getHashCode", &Vec::new(), "I", &Vec::new()), Variable::Int(0));
    }

    #[test]
    fn string_intern() {
        assert_eq!(void_int_call("tests/string.class", "intern"), 0x2);
    }

    #[test]
    fn builtins_reflection() {
        assert_eq!(void_int_call("tests/builtins_reflection.class", "getCallerClassTest"), 0x1);
    }

    #[test]
    fn class_getDeclared() {
        assert_eq!(void_int_call("tests/clazz.class", "getDeclaredFieldsTest"), 0x3);
    }

    fn void_int_call(path: &str, method: &str) -> i32 {
        return run_method(Path::new(path), method, &Vec::new(), "I", &vec!(String::from("./tests/"))).to_int();
    }

    fn int_int_call(path: &str, method: &str, arg: i32) -> i32 {
        return run_method(Path::new(path), method, &vec!(Variable::Int(arg)), "I", &vec!(String::from("./tests/"))).to_int();
    }

    fn int2_int_call(path: &str, method: &str, arg: i32, arg2: i32) -> i32 {
        return run_method(Path::new(path), method, &vec!(Variable::Int(arg), Variable::Int(arg2)), "I", &vec!(String::from("./tests/"))).to_int();
    }

    fn long2_long_call(path: &str, method: &str, arg: i64, arg2: i64) -> i64 {
        return run_method(Path::new(path), method, &vec!(Variable::Long(arg), Variable::Long(arg2)), "J", &vec!(String::from("./tests/"))).to_long();
    }

    fn float2_float_call(path: &str, method: &str, arg: f32, arg2: f32) -> f32 {
        return run_method(Path::new(path), method, &vec!(Variable::Float(arg), Variable::Float(arg2)), "F", &vec!(String::from("./tests/"))).to_float();
    }

    fn double2_double_call(path: &str, method: &str, arg: f64, arg2: f64) -> f64 {
        return run_method(Path::new(path), method, &vec!(Variable::Double(arg), Variable::Double(arg2)), "D", &vec!(String::from("./tests/"))).to_double();
    }

    #[test]
    fn arrays() {
        assert_eq!(void_int_call("tests/arrays.class", "arrayReturningFunctionTest"), 12);
        assert_eq!(void_int_call("tests/arrays.class", "arrayComparison"), 10);
    }

    #[test]
    fn inheritance() {
        assert_eq!(void_int_call("tests/inheritance.class", "basicImplementation"), 1);
        assert_eq!(void_int_call("tests/inheritance.class", "basicImplementationExtension"), 2);
        assert_eq!(void_int_call("tests/inheritance.class", "basicExtension"), 0x3987);
        assert_eq!(void_int_call("tests/inheritance.class", "basicImplementationDowncast"), 3);
        assert_eq!(void_int_call("tests/inheritance.class", "extendedMultipleImls"), 0x403);
    }

    #[test]
    fn multideps() {
        assert_eq!(void_int_call("tests/multideps.class", "test"), 5);
    }

    #[test]
    fn hash() {
        let hashes = vec!(
                         int_int_call("tests/hash.class", "hashA", 1),
                         int_int_call("tests/hash.class", "hashA", 2),
                         int2_int_call("tests/hash.class", "hashB", 1, 0),
                         int2_int_call("tests/hash.class", "hashB", 1, 1),
                         int2_int_call("tests/hash.class", "hashB", 2, 0),
                         int2_int_call("tests/hash.class", "hashC", 0, 0),
                         int2_int_call("tests/hash.class", "hashC", 0, 1),
                         int2_int_call("tests/hash.class", "hashC", 1, 0),
                         int2_int_call("tests/hash.class", "circularHashD", 1, 0),
                         int2_int_call("tests/hash.class", "circularHashD", 2, 0),
                         int2_int_call("tests/hash.class", "circularHashE", 1, 0),
                         int2_int_call("tests/hash.class", "circularHashE", 2, 0),
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
        assert_eq!(void_int_call("tests/static_init.class", "getx"), 1);
    }
}