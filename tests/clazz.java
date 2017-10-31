import java.lang.reflect.Field;
import java.lang.IllegalAccessException;
import sun.misc.Unsafe;

public class clazz {
    private int x = 0;
    private int y = 0;

    private static int getNumberOfFields() {
        return clazz.class.getDeclaredFields().length;
    }

    private static String getNameOfFirstField() {
        return clazz.class.getDeclaredFields()[0].getName();
    }

    private static boolean checkSlots() {
        return Unsafe.getUnsafe().objectFieldOffset(clazz.class.getDeclaredFields()[0]) != Unsafe.getUnsafe().objectFieldOffset(clazz.class.getDeclaredFields()[1]);
    }
}
