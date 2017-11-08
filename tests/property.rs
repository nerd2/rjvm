mod common;
use common::*;

#[test]
pub fn get_property() {
    let (mut runtime, class_path) = setup("get_property", r##"
        public class get_property {
            public static String get(String prop) {
                return System.getProperty(prop);
            }
            public static void set(String prop, String value) {
                System.setProperty(prop, value);
            }
        }
    "##);
    assert!(str_str_call(&mut runtime, class_path.as_path(), "get", "abcd").is_none());
    assert_eq!(str_str_call(&mut runtime, class_path.as_path(), "get", "file.encoding").unwrap(), "us-ascii");
    str2_void_call(&mut runtime, class_path.as_path(), "set", "file.encoding", "abc");
    assert_eq!(str_str_call(&mut runtime, class_path.as_path(), "get", "file.encoding").unwrap(), "abc");
}


