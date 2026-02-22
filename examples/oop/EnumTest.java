class EnumTest {
    enum Color {
        RED, GREEN, BLUE
    }

    public static void main(String[] args) {
        Color c = Color.RED;
        System.out.println(c.name());
        System.out.println(c.ordinal());

        Color g = Color.GREEN;
        System.out.println(g.name());
        System.out.println(g.ordinal());

        Color b = Color.BLUE;
        System.out.println(b.name());
        System.out.println(b.ordinal());
    }
}
