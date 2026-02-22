import java.util.List;

class ListOfTest {
    public static void main(String[] args) {
        var nums = List.of(1, 2, 3, 4, 5);

        // sorted + forEach
        nums.stream()
            .sorted()
            .forEach(x -> System.out.println(x));

        System.out.println("---");

        // anyMatch / allMatch / noneMatch
        System.out.println(nums.stream().anyMatch(x -> x > 3));
        System.out.println(nums.stream().allMatch(x -> x > 0));
        System.out.println(nums.stream().noneMatch(x -> x > 10));

        System.out.println("---");

        // distinct + count
        var dups = List.of(1, 2, 2, 3, 3, 3);
        System.out.println(dups.stream().distinct().count());

        // findFirst
        System.out.println(nums.stream().filter(x -> x > 3).findFirst());

        // limit + skip
        nums.stream().skip(2).limit(2).forEach(x -> System.out.println(x));
    }
}
