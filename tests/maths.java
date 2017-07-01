public class maths {
    public static short shortAddSubMulDivMod (short a, short b, short c, short d, short e, short f) {
        return (short)(((((a + b) - c) * (-d)) / e) % f);
    }

    public static int intAddSubMulDivMod (int a, int b, int c, int d, int e, int f) {
        return ((((a + b) - c) * (-d)) / e) % f;
    }

    public static long longAddSubMulDivMod (long a, long b, long c, long d, long e, long f) {
        return ((((a + b) - c) * (-d)) / e) % f;
    }

    public static float floatAddSubMulDivMod (float a, float b, float c, float d, float e, float f) {
        return ((((a + b) - c) * (-d)) / e) % f;
    }

    public static double doubleAddSubMulDivMod (double a, double b, double c, double d, double e, double f) {
        return ((((a + b) - c) * (-d)) / e) % f;
    }
}
