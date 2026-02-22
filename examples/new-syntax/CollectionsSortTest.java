import java.util.ArrayList;
import java.util.Collections;

class CollectionsSortTest {
    public static void main(String[] args) {
        // Sort integers
        ArrayList<Integer> nums = new ArrayList<>();
        nums.add(5);
        nums.add(2);
        nums.add(8);
        nums.add(1);
        nums.add(4);
        Collections.sort(nums);
        System.out.println(nums.toString());

        // Sort strings
        ArrayList<String> words = new ArrayList<>();
        words.add("banana");
        words.add("apple");
        words.add("cherry");
        Collections.sort(words);
        System.out.println(words.toString());

        // Sort with Comparator (reverse order)
        Collections.sort(nums, (a, b) -> b - a);
        System.out.println(nums.toString());
    }
}
