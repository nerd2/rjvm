extern crate rjvm;
extern crate glob;

#[cfg(test)]
mod tests {
    use rjvm::run_method;
    use rjvm::reader::runner::Variable;
    use glob::glob;
    use std::path::Path;

    #[test]
    fn maths() {
        assert_eq!(run_method(
            Path::new("tests/maths.class"),
            "intAddSubMulDivMod",
            &vec!(Variable::Int(11), Variable::Int(17), Variable::Int(3), Variable::Int(19), Variable::Int(5), Variable::Int(23)),
            Some(&Variable::Int(0))),
        Variable::Int(3));
    }
}