class PrintfTest {
    public static void main(String[] args) {
        System.out.printf("Hello, %s!%n", "World");
        System.out.printf("Number: %d%n", 42);
        System.out.printf("%s is %d years old%n", "Alice", 30);

        String formatted = String.format("%s scored %d points", "Bob", 95);
        System.out.println(formatted);
    }
}
