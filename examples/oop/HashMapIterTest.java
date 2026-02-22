import java.util.HashMap;

class HashMapIterTest {
    public static void main(String[] args) {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", 1);
        map.put("b", 2);

        // Iterate over entrySet
        for (var entry : map.entrySet()) {
            System.out.println(entry.getKey() + "=" + entry.getValue());
        }

        // size check
        System.out.println(map.size());
    }
}
