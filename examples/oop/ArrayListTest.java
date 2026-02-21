class ArrayListTest {
    public static void main(String[] args) {
        ArrayList<String> list = new ArrayList<>();
        list.add("hello");
        list.add("world");
        list.add("java");

        System.out.println(list.size());
        System.out.println(list.get(0));
        System.out.println(list.get(1));
        System.out.println(list.contains("java"));
        System.out.println(list.contains("rust"));

        list.remove(1);
        System.out.println(list.size());
        System.out.println(list.get(1));
    }
}
