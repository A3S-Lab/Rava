import java.util.ArrayList;

class StreamTest {
    public static void main(String[] args) {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(2);
        list.add(3);
        list.add(4);
        list.add(5);

        // map + forEach
        list.stream()
            .map(x -> x * 2)
            .forEach(x -> System.out.println(x));

        System.out.println("---");

        // filter + count
        var count = list.stream()
            .filter(x -> x % 2 == 0)
            .count();
        System.out.println(count);

        System.out.println("---");

        // reduce
        var sum = list.stream()
            .reduce(0, (a, b) -> a + b);
        System.out.println(sum);

        System.out.println("---");

        // chained: filter + map + forEach
        list.stream()
            .filter(x -> x > 2)
            .map(x -> x * 10)
            .forEach(x -> System.out.println(x));
    }
}
