extern crate rjvm;
extern crate glob;

#[cfg(test)]
mod tests {
    use rjvm::run;
    use glob::glob;

    #[test]
    fn expectation_checker() {
        for file in glob("tests/*.class").unwrap().filter_map(Result::ok) {
            run(&file);
        }
    }
}