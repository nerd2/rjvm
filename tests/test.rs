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
        assert_eq!(run_method(Path::new("tests/maths.class"), "add", &vec!(Variable::Int(1), Variable::Int(2)), Some(&Variable::Int(0))), Variable::Int(3));
    }
}