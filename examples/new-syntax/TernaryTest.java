class TernaryTest {
    public static void main(String[] args) {
        int x = 10;
        String result = x > 5 ? "big" : "small";
        System.out.println(result);

        // Nested ternary
        int y = 0;
        String sign = y > 0 ? "positive" : y < 0 ? "negative" : "zero";
        System.out.println(sign);

        // Ternary in expression
        System.out.println("value is " + (x % 2 == 0 ? "even" : "odd"));
    }
}
