mod common;
use common::*;

#[test]
fn static_init() {
    let (mut runtime, class_path) = setup("static_init", r##"
        public class static_init {
            static int x;

            static {
                x = 1;
            }

            static int getx() {
                return x;
            }
        }
    "##);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "getx"), 1);
}