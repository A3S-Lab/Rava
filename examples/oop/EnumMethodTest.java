enum Color {
    RED, GREEN, BLUE;

    String describe() {
        return "Color: " + this.name();
    }

    static Color fromOrdinal(int ord) {
        Color[] all = Color.values();
        return all[ord];
    }
}

class EnumMethodTest {
    public static void main(String[] args) {
        Color r = Color.RED;
        System.out.println(r.describe());
        System.out.println(Color.GREEN.describe());

        Color c = Color.fromOrdinal(2);
        System.out.println(c.name());
    }
}
