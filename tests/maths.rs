#[macro_use] extern crate assert_approx_eq;
mod common;
use common::*;

fn add_sub_mul_div_mod_test<F>(runtime: &mut Runtime, class_path: &Path, fn_name: &str, transform: F) where F: Fn(i32) -> Variable {
    let args = vec!(transform(11), transform(17), transform(3), transform(19), transform(5), transform(23));
    assert_eq!(run_method(runtime,
                          class_path,
                          fn_name,
                          &args,
                          transform(0).get_descriptor().as_str()
    ),
               transform(-3));
}

fn shift_test<F>(runtime: &mut Runtime, class_path: &Path, fn_name: &str, transform: F, result: i64) where F: Fn(i64) -> Variable {
    let args = vec!(transform(-3), transform(4), transform(2), transform(2));
    assert_eq!(run_method(runtime,
                          class_path,
                          fn_name,
                          &args,
                          transform(0).get_descriptor().as_str()
    ),
               transform(result));
}

#[test]
fn maths() {
    let (mut runtime, class_path) = setup("maths", r##"
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

            public static int signCheck(int ret) {
                for(int i = -5; i < 33; i++){
                    ret += i;
                }
                return ret;
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
    "##);
    assert_eq!(int_int_call(&mut runtime, class_path.as_path(), "signCheck", 123), 636);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intAdd", 1, 2), 3);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intAdd", 0x7FFFFFFF, 2), -0x7FFFFFFF);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intSub", 123, 2), 121);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intSub", -0x7FFFFFFF, 2), 0x7FFFFFFF);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intMul", 0x10100100, 0x1001), 0x10200100);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intDiv", 6, 3), 2);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intDiv", <i32>::min_value(), -1), <i32>::min_value());
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intRem", 6, 4), 2);
    assert_eq!(int2_int_call(&mut runtime, class_path.as_path(), "intRem", <i32>::min_value(), -1), 0);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longAdd", 0x123123123, 0x121212121), 0x244335244);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longAdd", 0x7FFFFFFFFFFFFFFF, 2), -0x7FFFFFFFFFFFFFFF);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longSub", 0x123123123, 0x123123120), 3);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longSub", -0x7FFFFFFFFFFFFFFF, 2), 0x7FFFFFFFFFFFFFFF);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longMul", 123, 100), 12300);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longMul", 0x1010010000000000, 0x1001), 0x1020010000000000);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longDiv", 1234, 2), 617);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longDiv", <i64>::min_value(), -1), <i64>::min_value());
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longRem", 1234, 3), 1);
    assert_eq!(long2_long_call(&mut runtime, class_path.as_path(), "longRem", <i64>::min_value(), -1), 0);
    assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatAdd", 1.1, 2.2), 3.3);
    assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatSub", 4.1, 2.2), 1.9);
    assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatMul", 1.1, 2.0), 2.2);
    assert_approx_eq!(float2_float_call(&mut runtime, class_path.as_path(), "floatDiv", 4.4, 1.1), 4.0);
    assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleAdd", 1.1, 2.2), 3.3);
    assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleSub", 4.1, 2.2), 1.9);
    assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleMul", 1.1, 2.0), 2.2);
    assert_approx_eq!(double2_double_call(&mut runtime, class_path.as_path(), "doubleDiv", 4.4, 1.1), 4.0);
    add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "shortAddSubMulDivMod", |x| Variable::Short(x as i16));
    add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "intAddSubMulDivMod", |x| Variable::Int(x as i32));
    add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "longAddSubMulDivMod", |x| Variable::Long(x as i64));
    add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "floatAddSubMulDivMod", |x| Variable::Float(x as f32));
    add_sub_mul_div_mod_test(&mut runtime, class_path.as_path(), "doubleAddSubMulDivMod", |x| Variable::Double(x as f64));
    shift_test(&mut runtime, class_path.as_path(), "intShlShrUshr", |x| Variable::Int(x as i32), 0x3FFFFFFD as i64);
    shift_test(&mut runtime, class_path.as_path(), "longShlShrUshr", |x| Variable::Long(x as i64), 0x3FFFFFFFFFFFFFFD as i64);
}
