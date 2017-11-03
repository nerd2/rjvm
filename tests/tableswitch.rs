mod common;
use common::*;

#[test]
fn tableswitch() {
    let (mut runtime, class_path) = setup("tableswitch", r##"
        public class tableswitch {
            public static boolean check(int x) {
                switch(x) {
                    case 11:
                    case 12:
                    case 13:
                    case 14:
                    case 15:
                        return true;
                    default:
                        return false;
                }
            }
        }
    "##);
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(1)), "Z"), Variable::Int(0));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(10)), "Z"), Variable::Int(0));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(11)), "Z"), Variable::Int(1));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(13)), "Z"), Variable::Int(1));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(15)), "Z"), Variable::Int(1));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(16)), "Z"), Variable::Int(0));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Int(130)), "Z"), Variable::Int(0));
}
