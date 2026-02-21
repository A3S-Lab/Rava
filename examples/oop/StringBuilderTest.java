class StringBuilderTest {
    public static void main(String[] args) {
        StringBuilder sb = new StringBuilder();
        sb.append("Hello");
        sb.append(", ");
        sb.append("World");
        sb.append("!");
        System.out.println(sb.toString());
        System.out.println(sb.length());

        StringBuilder sb2 = new StringBuilder();
        sb2.append("abc");
        sb2.reverse();
        System.out.println(sb2.toString());
    }
}
