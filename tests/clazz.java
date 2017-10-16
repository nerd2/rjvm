import java.lang.reflect.Field;
import java.lang.IllegalAccessException;

public class clazz {
    private int x = 0;
    private int y = 0;

    private static int getDeclaredFieldsTest() {
        clazz a = new clazz();

        Field[] fields = clazz.class.getDeclaredFields();
        int x = 1;
        for (Field f : fields) {
            f.setAccessible(true);
            try {
                f.setInt(a, x);
            } catch (IllegalAccessException e) {

            }
            x++;
        }

        return a.x + a.y;
    }
}
