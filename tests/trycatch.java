public class trycatch {
    private static class A {
        public static A a;

        public int x = 1;
    }

    public static int test() {
        try {
            return A.a.x;
        } catch (NullPointerException e) {
            return 2;
        }
    }
}
