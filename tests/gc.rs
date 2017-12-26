mod common;
use common::*;

#[ignore]
#[test]
fn gc() {
    let (mut runtime, class_path) = setup("gc", r##"
        public class gc {
            private static class A {
                public A a;
            }

            private static void createLoose() {
                //A p = new A();
                //A q = new A();
                //p.a = q;
                //q.a = p;
            }

            private static long basic() {
                long freeMemA = Runtime.getRuntime().freeMemory();

                createLoose();

                System.gc();

                long freeMemB = Runtime.getRuntime().freeMemory();

                return freeMemA - freeMemB;
            }
        }
    "##, false);

    assert_eq!(void_long_call(&mut runtime, class_path.as_path(), "basic"), 0);
}
