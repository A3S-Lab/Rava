class MultiCaseTest {
    public static void main(String[] args) {
        // Multi-case labels with arrow syntax
        int day = 3;
        String type = switch (day) {
            case 1, 2, 3, 4, 5 -> "Weekday";
            case 6, 7 -> "Weekend";
            default -> "Unknown";
        };
        System.out.println(type);

        // Multi-case in switch statement
        int x = 2;
        switch (x) {
            case 1, 2 -> System.out.println("one or two");
            case 3, 4 -> System.out.println("three or four");
            default -> System.out.println("other");
        }

        // Single case still works
        String s = switch (x) {
            case 1 -> "one";
            case 2 -> "two";
            default -> "other";
        };
        System.out.println(s);
    }
}
