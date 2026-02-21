class Fibonacci {
    public static void main(String[] args) {
        int n = 10;
        int a = 0;
        int b = 1;
        int i = 0;
        while (i < n) {
            System.out.println(a);
            int tmp = a + b;
            a = b;
            b = tmp;
            i = i + 1;
        }
    }
}
