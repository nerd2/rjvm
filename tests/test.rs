extern crate rjvm;
extern crate glob;

#[cfg(test)]
mod tests {
    use rjvm::run_method;
    use rjvm::reader::runner::Variable;
    use glob::glob;
    use std::path::Path;

    fn add_sub_mul_div_mod_test<F>(fn_name: &str, transform: F) where F: Fn(i32) -> Variable {
        let args = vec!(transform(11), transform(17), transform(3), transform(19), transform(5), transform(23));
        assert_eq!(run_method(
            Path::new("tests/maths.class"),
            fn_name,
            &args,
            Some(&transform(0)),
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
            Some(&transform(0)),
            &Vec::new()),
        transform(result));
    }

    #[test]
    fn maths() {
        add_sub_mul_div_mod_test("shortAddSubMulDivMod", |x| Variable::Short(x as i16));
        add_sub_mul_div_mod_test("intAddSubMulDivMod", |x| Variable::Int(x as i32));
        add_sub_mul_div_mod_test("longAddSubMulDivMod", |x| Variable::Long(x as i64));
        add_sub_mul_div_mod_test("floatAddSubMulDivMod", |x| Variable::Float(x as f32));
        add_sub_mul_div_mod_test("doubleAddSubMulDivMod", |x| Variable::Double(x as f64));
        shift_test("intShlShrUshr", |x| Variable::Int(x as i32), 0x3FFFFFFD as i64);
        shift_test("longShlShrUshr", |x| Variable::Long(x as i64), 0x3FFFFFFFFFFFFFFD as i64);
    }

    #[test]
    fn string() {
        assert_eq!(run_method(Path::new("tests/string.class"), "newAppendExtract", &Vec::new(), Some(&Variable::Char('\0')), &Vec::new()), Variable::Char('a'));
    }

    fn void_int_call(path: &str, method: &str) -> i32 {
        return run_method(Path::new(path), method, &Vec::new(), Some(&Variable::Int(0)), &vec!(String::from("/Users/sam/Personal/rjvm/tests/"))).to_int();
    }

    #[test]
    fn inheritance() {
        assert_eq!(void_int_call("tests/inheritance.class", "basicImplementation"), 1);
        assert_eq!(void_int_call("tests/inheritance.class", "basicImplementationExtension"), 2);
        assert_eq!(void_int_call("tests/inheritance.class", "basicExtension"), 0x3987);
        assert_eq!(void_int_call("tests/inheritance.class", "basicImplementationDowncast"), 3);
    }
}