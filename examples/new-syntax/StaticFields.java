class StaticFields {
    static int count = 0;
    static String name = "rava";

    static void increment() {
        StaticFields.count = StaticFields.count + 1;
    }

    public static void main(String[] args) {
        System.out.println(StaticFields.count);
        System.out.println(StaticFields.name);
        increment();
        increment();
        increment();
        System.out.println(StaticFields.count);
    }
}
