enum Color {
    RED("red", 1),
    GREEN("green", 2),
    BLUE("blue", 3);

    String label;
    int code;

    Color(String label, int code) {
        this.label = label;
        this.code = code;
    }

    String describe() {
        return this.label + "=" + this.code;
    }
}

class EnumFieldTest {
    public static void main(String[] args) {
        System.out.println(Color.RED.label);
        System.out.println(Color.GREEN.code);
        System.out.println(Color.BLUE.describe());
        System.out.println(Color.RED.name());
        System.out.println(Color.GREEN.ordinal());
    }
}
