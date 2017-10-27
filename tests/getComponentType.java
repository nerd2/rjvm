import java.lang.String;

public class getComponentType {
    private static class A {int x;}

    public static String getComponentTypeCheck1() {
        A[] a = new A[1];
        return a.getClass().getComponentType().getName();
    }

    public static String getComponentTypeCheck2() {
        boolean[] a = new boolean[1];
        return a.getClass().getComponentType().getName();
    }

}
