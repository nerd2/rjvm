mod common;
use common::*;

#[test]
pub fn compare_and_swap() {
    let (mut runtime, class_path) = setup("compare_and_swap", r##"
        import sun.misc.Unsafe;
        import java.lang.reflect.Field;
        import java.lang.NoSuchFieldException;
        import java.lang.RuntimeException;

        public class compare_and_swap {
            private static class A {
                public Object obj;
            }

            public static boolean compareAndSwapObject(boolean initWithObject, boolean compareWithObject, boolean swapWithObject) {
                A a = new A();
                a.obj = initWithObject ? new Object() : null;
                Field f;
                try {
                    f = a.getClass().getField("obj");
                } catch (NoSuchFieldException e) {
                    throw new RuntimeException(e);
                }
                long offset = Unsafe.getUnsafe().objectFieldOffset(f);
                Unsafe.getUnsafe().compareAndSwapObject(a, offset, compareWithObject ? a.obj : null, swapWithObject ? new Object() : null);
                return a.obj == null;
            }

            private static class B {
                public int i;
            }

            public static int compareAndSwapInt(int init, int compare, int swap) {
                B b = new B();
                b.i = init;
                Field f;
                try {
                    f = b.getClass().getField("i");
                } catch (NoSuchFieldException e) {
                    throw new RuntimeException(e);
                }
                long offset = Unsafe.getUnsafe().objectFieldOffset(f);
                Unsafe.getUnsafe().compareAndSwapInt(b, offset, compare, swap);
                return b.i;
            }

            private static class C {
                public long l;
            }

            public static long compareAndSwapLong(long init, long compare, long swap) {
                C c = new C();
                c.l = init;
                Field f;
                try {
                    f = c.getClass().getField("l");
                } catch (NoSuchFieldException e) {
                    throw new RuntimeException(e);
                }
                long offset = Unsafe.getUnsafe().objectFieldOffset(f);
                Unsafe.getUnsafe().compareAndSwapLong(c, offset, compare, swap);
                return c.l;
            }
        }
    "##);
    assert_eq!(bool3_bool_call(&mut runtime, class_path.as_path(), "compareAndSwapObject", false, false, true), false);
    assert_eq!(bool3_bool_call(&mut runtime, class_path.as_path(), "compareAndSwapObject", true, false, false), false);
    assert_eq!(bool3_bool_call(&mut runtime, class_path.as_path(), "compareAndSwapObject", true, true, false), true);

    assert_eq!(int3_int_call(&mut runtime, class_path.as_path(), "compareAndSwapInt", 5, 5, 10), 10);
    assert_eq!(int3_int_call(&mut runtime, class_path.as_path(), "compareAndSwapInt", 5, 6, 10), 5);

    assert_eq!(long3_long_call(&mut runtime, class_path.as_path(), "compareAndSwapLong", 5, 5, 10), 10);
    assert_eq!(long3_long_call(&mut runtime, class_path.as_path(), "compareAndSwapLong", 5, 6, 10), 5);
}
