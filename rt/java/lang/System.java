package java.lang;

import java.io.InputStream;
import java.io.PrintStream;

class System {
    public static final InputStream in = null;
    public static final PrintStream out = null;
    public static final PrintStream err = null;
    private static SecurityManager security = null;

    public static void arraycopy(Object src, int srcPos, Object dest, int destPos, int len) {
        Object[] srcArray = (Object[])src;
        Object[] destArray = (Object[])dest;
        while (len > 0) {
            destArray[destPos] = srcArray[srcPos];
            srcPos++;
            destPos++;
            len--;
        }
    }

    public static SecurityManager getSecurityManager() {
        return security;
    }
}
