mod common;
use common::*;

#[test]
fn get_component_type() {
    let (mut runtime, class_path) = setup("getComponentType", r##"
        import java.lang.String;

        public class getComponentType {
            private static class A {int x;}

            public static String getComponentTypeCheck1() {
                A[] a = new A[1];
                return a.getClass().getComponentType().getName();
            }

            public static String getComponentTypeCheck2() {
                boolean[] a = new boolean[1];
                return a.getClass().getComponentType().getName();
            }

        }
    "##, false);
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "getComponentTypeCheck1", &Vec::new(), "Ljava/lang/String;").extract_string(), "getComponentType$A");
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "getComponentTypeCheck2", &Vec::new(), "Ljava/lang/String;").extract_string(), "boolean");
}
