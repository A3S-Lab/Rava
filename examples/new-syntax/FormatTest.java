class FormatTest {
    public static void main(String[] args) {
        // Basic format
        System.out.println(String.format("Hello, %s!", "World"));
        System.out.println(String.format("%d + %d = %d", 1, 2, 3));

        // Float precision
        System.out.println(String.format("%.2f", 3.14159));
        System.out.println(String.format("%.0f", 3.7));
        System.out.println(String.format("%.4f", 1.0));

        // Width
        System.out.println(String.format("%10s", "right"));
        System.out.println(String.format("%-10s|", "left"));

        // Zero-padded
        System.out.println(String.format("%05d", 42));
    }
}
