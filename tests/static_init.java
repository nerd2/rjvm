public class static_init {
    static int x;

    static {
        x = 1;
    }

    static int getx() {
        return x;
    }
}
