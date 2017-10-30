public class multideps {
    public static class A {
        public static B b;
        public static int y;

        static {
            y = B.x;
        }
    }

    public static class B {
        public static int x = 4;
        public int z = 5;
    }

    public static class Root {
        public static A a;
        public static B b;
    }

    public static int test() {
        if (A.b != null) {
            return A.b.z;
        } else {
            return A.y;
        }
    }
}
