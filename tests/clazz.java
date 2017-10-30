import java.lang.reflect.Field;
import java.lang.IllegalAccessException;

public class clazz {
    private int x = 0;
    private int y = 0;

    private static int getNumberOfFields() {
        return clazz.class.getDeclaredFields().length;
    }

    private static String getNameOfFirstField() {
        return clazz.class.getDeclaredFields()[0].getName();
    }
}
