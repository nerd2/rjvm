mod common;
use common::*;

#[test]
fn binary_converter() {
    let (mut runtime, class_path) = setup("BinaryConverter", r##"
        // https://www.cs.utexas.edu/~scottm/cs307/javacode/codeSamples/BinaryConverter.java
        public class BinaryConverter {

            public static boolean test(){
                for(int i = -5; i < 33; i++){
                    System.out.println(i + ": " + toBinary(i));
                    System.out.println(i);
                    //always another way
                    System.out.println(i + ": " + Integer.toBinaryString(i));
                }
                return true;
            }

            /*
             * pre: none
             * post: returns a String with base10Num in base 2
             */
            public static String toBinary(int base10Num){
                boolean isNeg = base10Num < 0;
                base10Num = Math.abs(base10Num);
                String result = "";

                while(base10Num > 1){
                    result = (base10Num % 2) + result;
                    base10Num /= 2;
                }
                assert base10Num == 0 || base10Num == 1 : "value is not <= 1: " + base10Num;

                result = base10Num + result;

                if( isNeg )
                    result = "-" + result;
                return result;
            }
        }
    "##, true);
    assert_eq!(run_method(&mut runtime, class_path.as_path(), "test", &Vec::new(), "Z"), Variable::Int(1));
    assert_eq!(runtime.stdout,
r##"-5: -101
-5
-5: 11111111111111111111111111111011
-4: -100
-4
-4: 11111111111111111111111111111100
-3: -11
-3
-3: 11111111111111111111111111111101
-2: -10
-2
-2: 11111111111111111111111111111110
-1: -1
-1
-1: 11111111111111111111111111111111
0: 0
0
0: 0
1: 1
1
1: 1
2: 10
2
2: 10
3: 11
3
3: 11
4: 100
4
4: 100
5: 101
5
5: 101
6: 110
6
6: 110
7: 111
7
7: 111
8: 1000
8
8: 1000
9: 1001
9
9: 1001
10: 1010
10
10: 1010
11: 1011
11
11: 1011
12: 1100
12
12: 1100
13: 1101
13
13: 1101
14: 1110
14
14: 1110
15: 1111
15
15: 1111
16: 10000
16
16: 10000
17: 10001
17
17: 10001
18: 10010
18
18: 10010
19: 10011
19
19: 10011
20: 10100
20
20: 10100
21: 10101
21
21: 10101
22: 10110
22
22: 10110
23: 10111
23
23: 10111
24: 11000
24
24: 11000
25: 11001
25
25: 11001
26: 11010
26
26: 11010
27: 11011
27
27: 11011
28: 11100
28
28: 11100
29: 11101
29
29: 11101
30: 11110
30
30: 11110
31: 11111
31
31: 11111
32: 100000
32
32: 100000
"##)
}
