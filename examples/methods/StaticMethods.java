class StaticMethods {
    static int square(int n) {
        return n * n;
    }

    static int add(int a, int b) {
        return a + b;
    }

    static int max(int a, int b) {
        if (a > b) { return a; }
        return b;
    }

    public static void main(String[] args) {
        System.out.println(square(5));
        System.out.println(add(3, 4));
        System.out.println(square(add(2, 3)));
        System.out.println(max(10, 7));
    }
}
