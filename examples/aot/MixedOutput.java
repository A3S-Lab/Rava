class Main {
    static int fib(int n) {
        if (n <= 1) { return n; }
        return fib(n - 1) + fib(n - 2);
    }

    public static void main(String[] args) {
        System.out.println("Fibonacci sequence:");
        int i = 0;
        while (i < 8) {
            System.out.println("fib(" + i + ") = " + fib(i));
            i = i + 1;
        }
        System.out.println("Done!");
    }
}
