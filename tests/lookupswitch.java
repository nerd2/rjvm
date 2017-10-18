public class lookupswitch {
    public static boolean check(char x) {
        switch(x) {
            case '.':
            case '/':
            case ':':
            case ';':
            case '<':
            case '>':
            case '[':
                return true;
            default:
                return false;
        }
    }
}
