mod common;
use common::*;

#[test]
fn arrays() {
    let (mut runtime, class_path) = setup("arrays", r##"
        public class arrays {
            private static arrays[] arrayReturningFunction() {
                return new arrays[12];
            }

            private static boolean compareArrays(arrays[] a, arrays[] b) {
                return a == b;
            }

            public static int arrayReturningFunctionTest() {
                return arrayReturningFunction().length;
            }

            public static int arrayComparison() {
                arrays[] a = new arrays[12];
                arrays[] b = new arrays[12];
                return (compareArrays(b, b) ? 8 : 0) | (compareArrays(b, a) ? 4 : 0) | (compareArrays(a, a) ? 2 : 0) | (compareArrays(a, b) ? 1 : 0);
            }
        }
    "##, false);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "arrayReturningFunctionTest"), 12);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "arrayComparison"), 10);
}
