public class hash {
    private static class A {
        public int a;
    }

    private static class B extends A {
        public int b;
    }

    private static class C {
        public A a;
        public B b;
    }

    private static int hashA(int a) {
        A obj = new A();
        obj.a = a;
        return obj.hashCode();
    }

    private static int hashB(int b, int a) {
        B obj = new B();
        obj.a = a;
        obj.b = b;
        return obj.hashCode();
    }

    private static int hashC(int b, int a) {
        C obj = new C();
        obj.a = new A();
        obj.a.a = a;
        obj.b = new B();
        obj.b.b = b;
        return obj.hashCode();
    }


    private static class D {
        public E e;
        int x;
    }

    private static class E {
        public D d;
        int y;
    }

    private static int circularHashD(int x, int y) {
        D d = new D();
        E e = new E();
        d.e = e;
        d.x = x;
        e.d = d;
        e.y = y;
        return d.hashCode();
    }

    private static int circularHashE(int x, int y) {
        D d = new D();
        E e = new E();
        d.e = e;
        d.x = x;
        e.d = d;
        e.y = y;
        return e.hashCode();
    }
}
