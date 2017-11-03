mod common;
use common::*;

#[test]
pub fn try_catch() {
    let (mut runtime, class_path) = setup("trycatch", r##"
        public class trycatch {
            private static class A {
                public static A a;

                public int x = 1;
            }

            public static int test() {
                try {
                    return A.a.x;
                } catch (NullPointerException e) {
                    return 2;
                }
            }
        }
    "##);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "test"), 0x2);
}
