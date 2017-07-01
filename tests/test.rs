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
            Some(&transform(0))),
        transform(-3));
    }

    fn shift_test<F>(fn_name: &str, transform: F, result: i64) where F: Fn(i64) -> Variable {
        let args = vec!(transform(-3), transform(4), transform(2), transform(2));
        assert_eq!(run_method(
            Path::new("tests/maths.class"),
            fn_name,
            &args,
            Some(&transform(0))),
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
}