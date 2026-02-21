class HashMapTest {
    public static void main(String[] args) {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("alice", 90);
        map.put("bob", 85);
        map.put("charlie", 92);

        System.out.println(map.get("alice"));
        System.out.println(map.get("bob"));
        System.out.println(map.containsKey("charlie"));
        System.out.println(map.containsKey("dave"));
        System.out.println(map.size());

        map.remove("bob");
        System.out.println(map.size());
        System.out.println(map.get("bob"));
    }
}
