import java.lang.String;
import java.nio.charset.Charset;

public class string {
    public static char newAppendExtract() {
        String s = "hello_world";
        s = s + "_and_friends";
        return s.charAt(12);
    }

    public static char copy() {
        String s = "hello_world";
        String s2 = new String(s);
        return s.charAt(4);
    }

    public static byte getBytes() {
        String s = "hello_world";
        return s.getBytes(Charset.forName("UTF-8"))[1];
    }

    public static int getHashCode() {
        String s = "hello_world";
        return s.hashCode();
    }

    public static int intern() {
        String a = "a";
        String b = "b";
        String c = "c";

        String ab1 = "ab";
        String ab2 = a + b;

        boolean test1 = ab1 == ab2; // Should be false
        boolean test2 = ab1.intern() == ab2.intern(); // Should be true
        boolean test3 = ab1.intern() == a.intern(); // Should be false

        return (test1 ? 1 : 0) + (test2 ? 2 : 0) + (test3 ? 4 : 0);
    }


}
