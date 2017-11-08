mod common;
use common::*;

#[test]
pub fn unsafe_compare_and_swap() {
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


#[test]
pub fn unsafe_get_volatile() {
    let (mut runtime, class_path) = setup("unsafe_get_volatile", r##"
        import sun.misc.Unsafe;
        import java.lang.reflect.Field;
        import java.lang.NoSuchFieldException;
        import java.lang.RuntimeException;

        public class unsafe_get_volatile {
            private static class A {
                public int i;
                public int j;
                public long k;
                public long l;
                public short m;
                public short n;
                public B p;
                public B q;
            }

            private static class B {
                public long x;
            }

            private static long getOffset(String field) {
                Field f;
                try {
                    f = A.class.getField(field);
                } catch (NoSuchFieldException e) {
                    throw new RuntimeException(e);
                }
                return Unsafe.getUnsafe().objectFieldOffset(f);
            }

            public static long get() {
                A a = new A();
                a.i = 1;
                a.j = 2;
                a.k = 3;
                a.l = 4;
                a.m = 5;
                a.n = 6;
                a.p = new B();
                a.p.x = 7;
                a.q = new B();
                a.q.x = 8;

                return
                    (Unsafe.getUnsafe().getIntVolatile(a, getOffset("i")) << 0) +
                    (Unsafe.getUnsafe().getIntVolatile(a, getOffset("j")) << 4) +
                    (Unsafe.getUnsafe().getLongVolatile(a, getOffset("k")) << 8) +
                    (Unsafe.getUnsafe().getLongVolatile(a, getOffset("l")) << 12) +
                    (Unsafe.getUnsafe().getShortVolatile(a, getOffset("m")) << 16) +
                    (Unsafe.getUnsafe().getShortVolatile(a, getOffset("n")) << 20) +
                    (((B)(Unsafe.getUnsafe().getObjectVolatile(a, getOffset("p")))).x << 24) +
                    (((B)(Unsafe.getUnsafe().getObjectVolatile(a, getOffset("q")))).x << 28);
            }
        }
    "##);
    assert_eq!(void_long_call(&mut runtime, class_path.as_path(), "get"), 0x87654321);
}


#[test]
pub fn unsafe_allocate() {
    let (mut runtime, class_path) = setup("unsafe_allocate", r##"
        import sun.misc.Unsafe;

        public class unsafe_allocate {
            public static int get() {
                long mem = Unsafe.getUnsafe().allocateMemory(100);
                Unsafe.getUnsafe().putLong(mem + 8, 0x0102030405060708L);
                int ret = Unsafe.getUnsafe().getByte(mem + 10);
                Unsafe.getUnsafe().freeMemory(mem);
                return ret;
            }
        }
    "##);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "get"), 0x06);
}
