enum Color {
    RED, GREEN, BLUE;

    String describe() {
        return "Color: " + this.name();
    }
}

class EnumMethodTest2 {
    public static void main(String[] args) {
        Color r = Color.RED;
        System.out.println(r.describe());

        Color g = Color.GREEN;
        System.out.println(g.describe());

        Color b = Color.BLUE;
        System.out.println(b.name());
        System.out.println(b.ordinal());
    }
}
