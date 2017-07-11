package java.lang;

class System {
    private static SecurityManager security = new SecurityManager();

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
