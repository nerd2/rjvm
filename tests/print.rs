mod common;
use common::*;

#[test]
pub fn print() {
    let (mut runtime, class_path) = setup("print", r##"
        public class print {
            public static void stdout(String x) {
                System.out.println(x);
            }
            public static void stderr(String x) {
                System.err.println(x);
            }
        }
    "##, true);
    str_void_call(&mut runtime, class_path.as_path(), "stdout", "abcdef");
    assert_eq!(runtime.stdout, "abcdef\n");
    str_void_call(&mut runtime, class_path.as_path(), "stderr", "123456789");
    assert_eq!(runtime.stderr, "123456789\n");
}
