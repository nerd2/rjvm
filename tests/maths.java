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

    public static byte byteAdd(byte a, byte b) { return (byte)(a + b); }
    public static byte byteSub(byte a, byte b) { return (byte)(a - b); }
    public static byte byteMul(byte a, byte b) { return (byte)(a * b); }
    public static byte byteDiv(byte a, byte b) { return (byte)(a / b); }
    public static byte byteRem(byte a, byte b) { return (byte)(a % b); }
    public static short shortAdd(short a, short b) { return (short)(a + b); }
    public static short shortSub(short a, short b) { return (short)(a - b); }
    public static short shortMul(short a, short b) { return (short)(a * b); }
    public static short shortDiv(short a, short b) { return (short)(a / b); }
    public static short shortRem(short a, short b) { return (short)(a % b); }
    public static int intAdd(int a, int b) { return a + b; }
    public static int intSub(int a, int b) { return a - b; }
    public static int intMul(int a, int b) { return a * b; }
    public static int intDiv(int a, int b) { return (int)(a / b); }
    public static int intRem(int a, int b) { return (int)(a % b); }
    public static long longAdd(long a, long b) { return a + b; }
    public static long longSub(long a, long b) { return a - b; }
    public static long longMul(long a, long b) { return a * b; }
    public static long longDiv(long a, long b) { return (long)(a / b); }
    public static long longRem(long a, long b) { return (long)(a % b); }
    public static float floatAdd(float a, float b) { return a + b; }
    public static float floatSub(float a, float b) { return a - b; }
    public static float floatMul(float a, float b) { return a * b; }
    public static float floatDiv(float a, float b) { return (float)(a / b); }
    public static float floatRem(float a, float b) { return (float)(a % b); }
    public static double doubleAdd(double a, double b) { return a + b; }
    public static double doubleSub(double a, double b) { return a - b; }
    public static double doubleMul(double a, double b) { return a * b; }
    public static double doubleDiv(double a, double b) { return (double)(a / b); }
    public static double doubleRem(double a, double b) { return (double)(a % b); }

    public static int intShlShrUshr(int a, int b, int c, int d) {
        return ((a << b) >> c) >>> d;
    }

    public static long longShlShrUshr(long a, long b, long c, long d) {
        return ((a << b) >> c) >>> d;
    }
}
