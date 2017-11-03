mod common;
use common::*;

#[test]
fn inheritance() {
    let (mut runtime, class_path) = setup("inheritance", r##"
        public class inheritance {
            private interface I {
                public int f_i();
            }

            private interface J {
                public int f_j();
            }

            private static class A implements I {
                public int a;
                public int f_a() { return a; }
                public int f_i() { return 1; }
            }

            private static class B extends A {
                public int b;
                public int f_b() { return b; }
                public int f_i() { return 2; }
            }

            private static class C extends B implements J {
                public int c;
                public int f_c() { return c; }
                public int f_i() { return 3; }
                public int f_j() { return 4; }
            }

            public static int basicImplementation() {
                A a = new A();
                return a.f_i();
            }

            public static int basicImplementationExtension() {
                B b = new B();
                return b.f_i();
            }

            public static int basicExtension() {
                C c = new C();
                c.c = 9;
                c.b = 8;
                c.a = 7;
                return (c.f_i() << 12) + (c.f_c() << 8) + (c.f_b() << 4) + c.f_a();
            }

            public static int basicImplementationDowncast() {
                C c = new C();
                A a = (A)c;
                return a.f_i();
            }

            private static int runOnI(I i) {
                return i.f_i();
            }

            private static int runOnJ(J j) {
                return j.f_j();
            }

            public static int extendedMultipleImls() {
                C c = new C();
                return (runOnJ(c) << 8) + runOnI(c);
            }
        }
    "##);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicImplementation"), 1);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicImplementationExtension"), 2);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicExtension"), 0x3987);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "basicImplementationDowncast"), 3);
    assert_eq!(void_int_call(&mut runtime, class_path.as_path(), "extendedMultipleImls"), 0x403);
}