public class multideps {
    public static class A {
        public static B[] b;
        public static C c;
        public static int y;

        static {
            c = new C();
            y = B.x + c.b;
        }
    }

    public static class B {
        public static int x = 4;
        public int z = 5;
    }

    public static class C {
        public static int a = 1;
        public int b = 2;
    }

    public static class Root {
        public static A a;
        public static B b;
    }

    private static B[] getBArray() {
        return A.b;
    }

    public static int test() {
        if (getBArray() != null) {
            return getBArray()[0].z;
        } else {
            return A.y;
        }
    }
}
