import sun.reflect.Reflection;

public class builtins_reflection {
    private static class testClass {
        public static String getCallerClass() {
            return Reflection.getCallerClass().getName();
        }
    }

    public static int getCallerClassTest() {
        return testClass.getCallerClass().equals("builtins_reflection") ? 1 : 0;
    }
}
