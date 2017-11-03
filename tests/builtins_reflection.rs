mod common;
use common::*;

#[test]
fn builtins_reflection() {
    let (mut runtime, class_path) = setup("builtins_reflection", r##"
        import sun.reflect.Reflection;

        public class builtins_reflection {
            private static class testClass {
                public static String getCallerClass() {
                    return Reflection.getCallerClass().getName();
                }
            }

            public static int getCallerClassTest() {
                return testClass.getCallerClass().equals("builtins_reflection") ? 1 : 0;
            }
        }
    "##);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "getCallerClassTest"), 0x1);
}