mod common;
use common::*;

#[test]
fn lookupswitch() {
    let (mut runtime, class_path) = setup("lookupswitch", r##"
        public class lookupswitch {
            public static boolean check(char x) {
                switch(x) {
                    case '.':
                    case '/':
                    case ':':
                    case ';':
                    case '<':
                    case '>':
                    case '[':
                        return true;
                    default:
                        return false;
                }
            }
        }
    "##, false);
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char('a')), "Z"), Variable::Int(0));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char('.')), "Z"), Variable::Int(1));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char('>')), "Z"), Variable::Int(1));
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "check", &vec!(Variable::Char(' ')), "Z"), Variable::Int(0));
}
