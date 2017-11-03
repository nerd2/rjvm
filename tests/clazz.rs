mod common;
use common::*;

#[test]
fn class_get_declared() {
    let (mut runtime, class_path) = setup("clazz", r##"
        import java.lang.reflect.Field;
        import java.lang.IllegalAccessException;
        import sun.misc.Unsafe;

        public class clazz {
            private int x = 0;
            private int y = 0;

            private static int getNumberOfFields() {
                return clazz.class.getDeclaredFields().length;
            }

            private static String getNameOfFirstField() {
                return clazz.class.getDeclaredFields()[0].getName();
            }

            private static boolean checkSlots() {
                return Unsafe.getUnsafe().objectFieldOffset(clazz.class.getDeclaredFields()[0]) != Unsafe.getUnsafe().objectFieldOffset(clazz.class.getDeclaredFields()[1]);
            }
        }
    "##);
    assert_eq!(void_bool_call(&mut runtime, class_path.as_path(), "checkSlots"), true);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "getNumberOfFields"), 0x2);
    assert_eq!(void_str_call(&mut runtime, class_path.as_path(), "getNameOfFirstField"), "x");
}
