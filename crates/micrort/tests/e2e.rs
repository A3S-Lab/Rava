//! End-to-end tests: compile Java source → run via RirInterpreter → check stdout.

use rava_frontend::Compiler;
use rava_micrort::RirInterpreter;

/// Compile and run a Java snippet, capturing stdout output.
fn run(src: &str) -> String {
    let compiler = Compiler::new();
    let module = compiler.compile(src, std::path::Path::new("Test.java"))
        .expect("compile failed");
    let interp = RirInterpreter::new(module);
    let mut output = Vec::new();
    interp.run_main_with_output(&mut output).expect("run failed");
    String::from_utf8(output).unwrap()
}

#[test]
fn hello_world() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn arithmetic() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int a = 10;
        int b = 3;
        System.out.println(a + b);
        System.out.println(a - b);
        System.out.println(a * b);
        System.out.println(a / b);
        System.out.println(a % b);
    }
}
"#);
    assert_eq!(out.trim(), "13\n7\n30\n3\n1");
}

#[test]
fn object_fields() {
    let out = run(r#"
class Point {
    int x;
    int y;
    Point(int x, int y) { this.x = x; this.y = y; }
    int sum() { return x + y; }
}
class Main {
    public static void main(String[] args) {
        Point p = new Point(3, 4);
        System.out.println(p.x);
        System.out.println(p.y);
        System.out.println(p.sum());
    }
}
"#);
    assert_eq!(out.trim(), "3\n4\n7");
}

#[test]
fn inheritance() {
    let out = run(r#"
class Animal {
    String name;
    Animal(String name) { this.name = name; }
    String speak() { return "..."; }
}
class Dog extends Animal {
    Dog(String name) { super(name); }
    String speak() { return "Woof"; }
}
class Main {
    public static void main(String[] args) {
        Animal a = new Dog("Rex");
        System.out.println(a.speak());
        System.out.println(a.name);
    }
}
"#);
    assert_eq!(out.trim(), "Woof\nRex");
}

#[test]
fn for_loop_and_array() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[] arr = {1, 2, 3, 4, 5};
        int sum = 0;
        for (int i = 0; i < arr.length; i++) {
            sum += arr[i];
        }
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "15");
}

#[test]
fn string_methods() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello, World!";
        System.out.println(s.length());
        System.out.println(s.toUpperCase());
        System.out.println(s.contains("World"));
        System.out.println(s.substring(7, 12));
    }
}
"#);
    assert_eq!(out.trim(), "13\nHELLO, WORLD!\ntrue\nWorld");
}

#[test]
fn arraylist() {
    let out = run(r#"
import java.util.ArrayList;
class Main {
    public static void main(String[] args) {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(10);
        list.add(20);
        list.add(30);
        System.out.println(list.size());
        System.out.println(list.get(1));
        list.remove(0);
        System.out.println(list.size());
    }
}
"#);
    assert_eq!(out.trim(), "3\n20\n2");
}

#[test]
fn recursion_fibonacci() {
    let out = run(r#"
class Main {
    static int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }
    public static void main(String[] args) {
        System.out.println(fib(10));
    }
}
"#);
    assert_eq!(out.trim(), "55");
}

#[test]
fn static_fields() {
    let out = run(r#"
class Counter {
    static int count = 0;
    static void increment() { count++; }
}
class Main {
    public static void main(String[] args) {
        Counter.increment();
        Counter.increment();
        Counter.increment();
        System.out.println(Counter.count);
    }
}
"#);
    assert_eq!(out.trim(), "3");
}

#[test]
fn try_catch() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        try {
            int x = 10 / 0;
            System.out.println("no exception");
        } catch (ArithmeticException e) {
            System.out.println("caught");
        }
    }
}
"#);
    assert_eq!(out.trim(), "caught");
}

#[test]
fn hashmap() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> map = new HashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.put("c", 3);
        System.out.println(map.get("b"));
        System.out.println(map.size());
        System.out.println(map.containsKey("a"));
        System.out.println(map.containsKey("z"));
        map.remove("a");
        System.out.println(map.size());
    }
}
"#);
    assert_eq!(out.trim(), "2\n3\ntrue\nfalse\n2");
}

#[test]
fn collections_sort_and_foreach() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = new ArrayList<>(Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6));
        Collections.sort(nums);
        System.out.println(nums.get(0));
        System.out.println(nums.get(nums.size() - 1));
        int sum = 0;
        for (int n : nums) { sum += n; }
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "1\n9\n31");
}

#[test]
fn string_format() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(String.format("%d + %d = %d", 3, 4, 7));
        System.out.println(String.format("Hello, %s!", "World"));
        System.out.println(String.format("%.2f", 3.14159));
    }
}
"#);
    assert_eq!(out.trim(), "3 + 4 = 7\nHello, World!\n3.14");
}

#[test]
fn interface_and_polymorphism() {
    let out = run(r#"
interface Shape {
    double area();
    default String describe() { return "shape with area " + area(); }
}
class Circle implements Shape {
    double r;
    Circle(double r) { this.r = r; }
    public double area() { return 3.14159 * r * r; }
}
class Rect implements Shape {
    int w, h;
    Rect(int w, int h) { this.w = w; this.h = h; }
    public double area() { return w * h; }
}
class Main {
    public static void main(String[] args) {
        Shape[] shapes = { new Rect(3, 4), new Rect(5, 6) };
        int total = 0;
        for (Shape s : shapes) { total += (int) s.area(); }
        System.out.println(total);
    }
}
"#);
    assert_eq!(out.trim(), "42");
}

#[test]
fn generic_method() {
    let out = run(r#"
import java.util.*;
class Main {
    static <T extends Comparable<T>> T max(T a, T b) {
        return a.compareTo(b) >= 0 ? a : b;
    }
    public static void main(String[] args) {
        System.out.println(max(3, 7));
        System.out.println(max("apple", "banana"));
    }
}
"#);
    assert_eq!(out.trim(), "7\nbanana");
}

#[test]
fn lambda_and_stream() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        int sum = nums.stream()
            .filter(n -> n % 2 == 0)
            .mapToInt(Integer::intValue)
            .sum();
        System.out.println(sum);
        long count = nums.stream().filter(n -> n > 5).count();
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "30\n5");
}

#[test]
fn enum_basic() {
    let out = run(r#"
enum Day { MON, TUE, WED, THU, FRI, SAT, SUN }
class Main {
    public static void main(String[] args) {
        Day d = Day.WED;
        System.out.println(d);
        System.out.println(d.ordinal());
        System.out.println(Day.values().length);
    }
}
"#);
    assert_eq!(out.trim(), "WED\n2\n7");
}

#[test]
fn switch_expression() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        for (int i = 1; i <= 5; i++) {
            String s = switch (i) {
                case 1 -> "one";
                case 2 -> "two";
                case 3 -> "three";
                default -> "other";
            };
            System.out.println(s);
        }
    }
}
"#);
    assert_eq!(out.trim(), "one\ntwo\nthree\nother\nother");
}

#[test]
fn treemap_sorted() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> map = new TreeMap<>();
        map.put("banana", 2);
        map.put("apple", 1);
        map.put("cherry", 3);
        for (String k : map.keySet()) System.out.println(k);
        System.out.println(map.get("apple"));
    }
}
"#);
    assert_eq!(out.trim(), "apple\nbanana\ncherry\n1");
}

#[test]
fn priority_queue() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        pq.offer(5); pq.offer(1); pq.offer(3); pq.offer(2);
        System.out.println(pq.poll());
        System.out.println(pq.poll());
        System.out.println(pq.size());
    }
}
"#);
    assert_eq!(out.trim(), "1\n2\n2");
}

#[test]
fn integer_radix() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Integer.toString(255, 16));
        System.out.println(Integer.toString(10, 2));
        System.out.println(Integer.toHexString(255));
        System.out.println(Integer.toBinaryString(10));
    }
}
"#);
    assert_eq!(out.trim(), "ff\n1010\nff\n1010");
}

#[test]
fn string_chars_stream() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "hello world";
        long vowels = s.chars().filter(c -> "aeiou".indexOf(c) >= 0).count();
        System.out.println(vowels);
        long spaces = s.chars().filter(c -> c == ' ').count();
        System.out.println(spaces);
    }
}
"#);
    assert_eq!(out.trim(), "3\n1");
}

#[test]
fn math_constants_and_abstract() {
    let out = run(r#"
abstract class Shape {
    abstract double area();
    String describe() { return String.format("%.2f", area()); }
}
class Circle extends Shape {
    double r;
    Circle(double r) { this.r = r; }
    public double area() { return Math.PI * r * r; }
}
class Main {
    public static void main(String[] args) {
        Shape c = new Circle(1.0);
        System.out.println(c.describe());
        System.out.println(Math.abs(-5));
        System.out.println(Math.max(3, 7));
        System.out.println((int) Math.pow(2, 8));
        System.out.println(Integer.MAX_VALUE > 0);
    }
}
"#);
    assert_eq!(out.trim(), "3.14\n5\n7\n256\ntrue");
}

#[test]
fn user_defined_tostring() {
    let out = run(r#"
class Point {
    int x, y;
    Point(int x, int y) { this.x = x; this.y = y; }
    public String toString() { return "(" + x + ", " + y + ")"; }
}
class Main {
    public static void main(String[] args) {
        Point p = new Point(3, 4);
        System.out.println(p);
        System.out.println("Point: " + p);
    }
}
"#);
    assert_eq!(out.trim(), "(3, 4)\nPoint: (3, 4)");
}

#[test]
fn nested_generics() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        Map<Integer, List<String>> grouped = Stream.of("a","bb","cc","ddd","e")
            .collect(Collectors.groupingBy(String::length));
        System.out.println(grouped.get(1).size());
        System.out.println(grouped.get(2).size());
        String joined = Stream.of("x","y","z").collect(Collectors.joining("-"));
        System.out.println(joined);
    }
}
"#);
    assert_eq!(out.trim(), "2\n2\nx-y-z");
}

#[test]
fn nested_for_each_2d() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] matrix = {{1,2,3},{4,5,6},{7,8,9}};
        int sum = 0;
        for (int[] row : matrix) for (int v : row) sum += v;
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "45");
}

#[test]
fn string_format_padding() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(String.format("%-5s|%5d", "hi", 42));
        System.out.println(String.format("%05d", 7));
    }
}
"#);
    assert_eq!(out.trim(), "hi   |   42\n00007");
}

#[test]
fn null_pointer_exception() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        try {
            String s = null;
            int len = s.length();
        } catch (NullPointerException e) {
            System.out.println("NPE");
        }
    }
}
"#);
    assert_eq!(out.trim(), "NPE");
}

#[test]
fn array_index_out_of_bounds() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        try {
            int[] arr = {1, 2, 3};
            int x = arr[5];
        } catch (ArrayIndexOutOfBoundsException e) {
            System.out.println("AIOOBE");
        }
    }
}
"#);
    assert_eq!(out.trim(), "AIOOBE");
}

#[test]
fn collectors_to_map() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> map = Stream.of("a", "bb", "ccc")
            .collect(Collectors.toMap(s -> s, s -> s.length()));
        System.out.println(map.get("a"));
        System.out.println(map.get("bb"));
        System.out.println(map.get("ccc"));
    }
}
"#);
    assert_eq!(out.trim(), "1\n2\n3");
}

#[test]
fn stream_flat_map() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<Integer> flat = Stream.of(Arrays.asList(1,2), Arrays.asList(3,4))
            .flatMap(l -> l.stream())
            .collect(Collectors.toList());
        System.out.println(flat.size());
        System.out.println(flat.get(0));
        System.out.println(flat.get(3));
    }
}
"#);
    assert_eq!(out.trim(), "4\n1\n4");
}

#[test]
fn stream_sorted_comparator() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> sorted = Stream.of("banana","apple","cherry")
            .sorted(Comparator.naturalOrder())
            .collect(Collectors.toList());
        System.out.println(sorted.get(0));
        System.out.println(sorted.get(2));
        List<String> rev = Stream.of("banana","apple","cherry")
            .sorted(Comparator.reverseOrder())
            .collect(Collectors.toList());
        System.out.println(rev.get(0));
    }
}
"#);
    assert_eq!(out.trim(), "apple\ncherry\ncherry");
}

#[test]
fn do_while_loop() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int i = 0, sum = 0;
        do {
            sum += i;
            i++;
        } while (i < 5);
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "10");
}

#[test]
fn string_switch() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String[] days = {"MON", "SAT", "WED"};
        for (String d : days) {
            switch (d) {
                case "SAT": case "SUN":
                    System.out.println("weekend");
                    break;
                default:
                    System.out.println("weekday");
            }
        }
    }
}
"#);
    assert_eq!(out.trim(), "weekday\nweekend\nweekday");
}

#[test]
fn string_builder() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        StringBuilder sb = new StringBuilder();
        sb.append("Hello");
        sb.append(", ");
        sb.append("World");
        sb.append("!");
        System.out.println(sb.toString());
        System.out.println(sb.length());
        sb.reverse();
        System.out.println(sb.toString());
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!\n13\n!dlroW ,olleH");
}

#[test]
fn multi_catch() {
    let out = run(r#"
class Main {
    static int parse(String s) { return Integer.parseInt(s); }
    public static void main(String[] args) {
        String[] inputs = {"42", "abc", null};
        for (String s : inputs) {
            try {
                int n = parse(s);
                System.out.println(n);
            } catch (NumberFormatException | NullPointerException e) {
                System.out.println("error");
            }
        }
    }
}
"#);
    assert_eq!(out.trim(), "42\nerror\nerror");
}

#[test]
fn try_with_resources() {
    let out = run(r#"
class Resource {
    String name;
    Resource(String name) { this.name = name; System.out.println("open " + name); }
    void close() { System.out.println("close " + name); }
    void use() { System.out.println("use " + name); }
}
class Main {
    public static void main(String[] args) {
        try (Resource r = new Resource("A")) {
            r.use();
        }
    }
}
"#);
    assert_eq!(out.trim(), "open A\nuse A\nclose A");
}

#[test]
fn labeled_break() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int count = 0;
        outer:
        for (int i = 0; i < 5; i++) {
            for (int j = 0; j < 5; j++) {
                if (i + j == 6) break outer;
                count++;
            }
        }
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "14");
}

#[test]
fn var_type_inference() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        var x = 42;
        var s = "hello";
        var list = new ArrayList<String>();
        list.add("a");
        list.add("b");
        System.out.println(x);
        System.out.println(s.toUpperCase());
        System.out.println(list.size());
    }
}
"#);
    assert_eq!(out.trim(), "42\nHELLO\n2");
}

#[test]
fn instanceof_pattern() {
    let out = run(r#"
class Main {
    static String describe(Object obj) {
        if (obj instanceof String s) {
            return "string of length " + s.length();
        } else if (obj instanceof Integer i) {
            return "integer " + i;
        }
        return "other";
    }
    public static void main(String[] args) {
        System.out.println(describe("hello"));
        System.out.println(describe(42));
        System.out.println(describe(3.14));
    }
}
"#);
    assert_eq!(out.trim(), "string of length 5\ninteger 42\nother");
}

#[test]
fn record_class() {
    let out = run(r#"
record Point(int x, int y) {
    int sum() { return x + y; }
}
class Main {
    public static void main(String[] args) {
        Point p = new Point(3, 4);
        System.out.println(p.x());
        System.out.println(p.y());
        System.out.println(p.sum());
    }
}
"#);
    assert_eq!(out.trim(), "3\n4\n7");
}

#[test]
fn switch_type_pattern() {
    let out = run(r#"
class Main {
    static String format(Object obj) {
        return switch (obj) {
            case Integer i -> "int:" + i;
            case String s -> "str:" + s;
            default -> "other";
        };
    }
    public static void main(String[] args) {
        System.out.println(format(42));
        System.out.println(format("hi"));
        System.out.println(format(3.14));
    }
}
"#);
    assert_eq!(out.trim(), "int:42\nstr:hi\nother");
}

#[test]
fn enum_with_fields_and_methods() {
    let out = run(r#"
enum Color {
    RED(255, 0, 0),
    GREEN(0, 255, 0),
    BLUE(0, 0, 255);

    private final int r;
    private final int g;
    private final int b;

    Color(int r, int g, int b) {
        this.r = r;
        this.g = g;
        this.b = b;
    }

    int brightness() { return r + g + b; }
}
class Main {
    public static void main(String[] args) {
        System.out.println(Color.values().length);
        System.out.println(Color.GREEN.name());
        System.out.println(Color.RED.ordinal());
        System.out.println(Color.BLUE.brightness());
    }
}
"#);
    assert_eq!(out.trim(), "3\nGREEN\n0\n255");
}

#[test]
fn varargs() {
    let out = run(r#"
class Main {
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }
    static String join(String sep, String... parts) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < parts.length; i++) {
            if (i > 0) sb.append(sep);
            sb.append(parts[i]);
        }
        return sb.toString();
    }
    public static void main(String[] args) {
        System.out.println(sum(1, 2, 3, 4, 5));
        System.out.println(join(", ", "a", "b", "c"));
    }
}
"#);
    assert_eq!(out.trim(), "15\na, b, c");
}

#[test]
fn static_initializer() {
    let out = run(r#"
class Config {
    static int MAX;
    static String PREFIX;
    static {
        MAX = 100;
        PREFIX = "cfg_";
    }
}
class Main {
    public static void main(String[] args) {
        System.out.println(Config.MAX);
        System.out.println(Config.PREFIX);
    }
}
"#);
    assert_eq!(out.trim(), "100\ncfg_");
}

#[test]
fn constructor_delegation() {
    let out = run(r#"
class Point {
    int x, y, z;
    Point(int x, int y) { this(x, y, 0); }
    Point(int x, int y, int z) { this.x = x; this.y = y; this.z = z; }
    public String toString() { return x + "," + y + "," + z; }
}
class Main {
    public static void main(String[] args) {
        Point p1 = new Point(1, 2);
        Point p2 = new Point(3, 4, 5);
        System.out.println(p1);
        System.out.println(p2);
    }
}
"#);
    assert_eq!(out.trim(), "1,2,0\n3,4,5");
}

#[test]
fn multi_dimensional_array() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] grid = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                grid[i][j] = i * 3 + j + 1;
        System.out.println(grid[0][0]);
        System.out.println(grid[1][1]);
        System.out.println(grid[2][2]);
        System.out.println(grid.length);
        System.out.println(grid[0].length);
    }
}
"#);
    assert_eq!(out.trim(), "1\n5\n9\n3\n3");
}

#[test]
fn assert_statement() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        try {
            int x = 5;
            assert x > 0 : "x must be positive";
            System.out.println("ok");
            assert x < 0 : "x must be negative";
            System.out.println("should not reach");
        } catch (AssertionError e) {
            System.out.println("assertion failed");
        }
    }
}
"#);
    assert_eq!(out.trim(), "ok\nassertion failed");
}

#[test]
fn inner_class() {
    let out = run(r#"
class Outer {
    int value;
    Outer(int v) { this.value = v; }
    class Inner {
        int doubled() { return value * 2; }
    }
}
class Main {
    public static void main(String[] args) {
        Outer o = new Outer(21);
        System.out.println(o.value);
    }
}
"#);
    assert_eq!(out.trim(), "21");
}

#[test]
fn comparable_and_sorting() {
    let out = run(r#"
import java.util.*;
class Student implements Comparable<Student> {
    String name;
    int grade;
    Student(String name, int grade) { this.name = name; this.grade = grade; }
    public int compareTo(Student other) { return Integer.compare(this.grade, other.grade); }
    public String toString() { return name + ":" + grade; }
}
class Main {
    public static void main(String[] args) {
        List<Student> students = new ArrayList<>();
        students.add(new Student("Alice", 85));
        students.add(new Student("Bob", 92));
        students.add(new Student("Charlie", 78));
        Collections.sort(students);
        for (Student s : students) System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "Charlie:78\nAlice:85\nBob:92");
}

#[test]
fn string_operations_advanced() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "  Hello, World!  ";
        System.out.println(s.strip());
        System.out.println(s.trim());
        System.out.println("abc".repeat(3));
        System.out.println(String.join("-", "a", "b", "c"));
        System.out.println("hello".indexOf("ll"));
        System.out.println("hello world".replace("world", "Java"));
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!\nHello, World!\nabcabcabc\na-b-c\n2\nhello Java");
}

#[test]
fn exception_hierarchy() {
    let out = run(r#"
class AppException extends RuntimeException {
    int code;
    AppException(String msg, int code) {
        super(msg);
        this.code = code;
    }
}
class Main {
    static void risky(int x) {
        if (x < 0) throw new AppException("negative", -1);
        if (x == 0) throw new ArithmeticException("zero");
        System.out.println("ok: " + x);
    }
    public static void main(String[] args) {
        try { risky(5); } catch (Exception e) { System.out.println("err"); }
        try { risky(-1); } catch (AppException e) { System.out.println("app:" + e.code); }
        try { risky(0); } catch (ArithmeticException e) { System.out.println("arith"); }
    }
}
"#);
    assert_eq!(out.trim(), "ok: 5\napp:-1\narith");
}

#[test]
fn optional() {
    let out = run(r#"
import java.util.Optional;
class Main {
    static Optional<String> find(String[] arr, String target) {
        for (String s : arr) {
            if (s.equals(target)) return Optional.of(s);
        }
        return Optional.empty();
    }
    public static void main(String[] args) {
        String[] words = {"hello", "world", "java"};
        Optional<String> found = find(words, "world");
        System.out.println(found.isPresent());
        System.out.println(found.get());
        Optional<String> missing = find(words, "rust");
        System.out.println(missing.isPresent());
        System.out.println(missing.orElse("not found"));
    }
}
"#);
    assert_eq!(out.trim(), "true\nworld\nfalse\nnot found");
}

#[test]
fn hashset_and_treeset() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Set<String> set = new HashSet<>();
        set.add("banana");
        set.add("apple");
        set.add("banana");
        System.out.println(set.size());
        System.out.println(set.contains("apple"));
        System.out.println(set.contains("cherry"));
        set.remove("apple");
        System.out.println(set.size());

        TreeSet<Integer> ts = new TreeSet<>();
        ts.add(5); ts.add(1); ts.add(3); ts.add(2); ts.add(4);
        System.out.println(ts.first());
        System.out.println(ts.last());
        System.out.println(ts.size());
    }
}
"#);
    assert_eq!(out.trim(), "2\ntrue\nfalse\n1\n1\n5\n5");
}

#[test]
fn arraydeque() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Deque<Integer> dq = new ArrayDeque<>();
        dq.addFirst(2);
        dq.addFirst(1);
        dq.addLast(3);
        dq.addLast(4);
        System.out.println(dq.peekFirst());
        System.out.println(dq.peekLast());
        System.out.println(dq.pollFirst());
        System.out.println(dq.pollLast());
        System.out.println(dq.size());
    }
}
"#);
    assert_eq!(out.trim(), "1\n4\n1\n4\n2");
}

#[test]
fn comparator_comparing() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Person {
    String name;
    int age;
    Person(String name, int age) { this.name = name; this.age = age; }
    String getName() { return name; }
    int getAge() { return age; }
    public String toString() { return name + ":" + age; }
}
class Main {
    public static void main(String[] args) {
        List<Person> people = new ArrayList<>();
        people.add(new Person("Charlie", 30));
        people.add(new Person("Alice", 25));
        people.add(new Person("Bob", 35));
        people.sort((a, b) -> Integer.compare(a.getAge(), b.getAge()));
        for (Person p : people) System.out.println(p);
    }
}
"#);
    assert_eq!(out.trim(), "Alice:25\nCharlie:30\nBob:35");
}

#[test]
fn map_entry_iteration() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> scores = new TreeMap<>();
        scores.put("Alice", 95);
        scores.put("Bob", 87);
        scores.put("Charlie", 92);
        int total = 0;
        for (Map.Entry<String, Integer> e : scores.entrySet()) {
            System.out.println(e.getKey() + "=" + e.getValue());
            total += e.getValue();
        }
        System.out.println(total);
    }
}
"#);
    assert_eq!(out.trim(), "Alice=95\nBob=87\nCharlie=92\n274");
}

#[test]
fn functional_interfaces() {
    let out = run(r#"
import java.util.function.*;
import java.util.*;
import java.util.stream.*;
class Main {
    static <T, R> List<R> transform(List<T> list, Function<T, R> fn) {
        List<R> result = new ArrayList<>();
        for (T item : list) result.add(fn.apply(item));
        return result;
    }
    static <T> List<T> filterList(List<T> list, Predicate<T> pred) {
        List<T> result = new ArrayList<>();
        for (T item : list) if (pred.test(item)) result.add(item);
        return result;
    }
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        List<Integer> doubled = transform(nums, x -> x * 2);
        System.out.println(doubled.get(0));
        System.out.println(doubled.get(4));
        List<Integer> evens = filterList(nums, x -> x % 2 == 0);
        System.out.println(evens.size());
        System.out.println(evens.get(0));
    }
}
"#);
    assert_eq!(out.trim(), "2\n10\n2\n2");
}

#[test]
fn math_methods() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println((int) Math.sqrt(16));
        System.out.println((int) Math.floor(3.9));
        System.out.println((int) Math.ceil(3.1));
        System.out.println(Math.round(3.5));
        System.out.println(Math.min(10, 20));
        System.out.println(Math.max(10, 20));
        System.out.println(Math.abs(-42));
        System.out.println((int) Math.pow(2, 10));
        System.out.println(Math.log(Math.E) > 0.99);
    }
}
"#);
    assert_eq!(out.trim(), "4\n3\n4\n4\n10\n20\n42\n1024\ntrue");
}

#[test]
fn nested_lambda_capture() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        int multiplier = 3;
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        List<Integer> result = nums.stream()
            .filter(n -> n % 2 != 0)
            .map(n -> n * multiplier)
            .collect(Collectors.toList());
        for (int n : result) System.out.println(n);
    }
}
"#);
    assert_eq!(out.trim(), "3\n9\n15");
}

#[test]
fn multi_level_inheritance() {
    let out = run(r#"
class A {
    String name() { return "A"; }
    String greet() { return "Hello from " + name(); }
}
class B extends A {
    String name() { return "B"; }
}
class C extends B {
    String name() { return "C"; }
    String extra() { return super.greet() + " via C"; }
}
class Main {
    public static void main(String[] args) {
        A a = new A();
        A b = new B();
        A c = new C();
        System.out.println(a.greet());
        System.out.println(b.greet());
        System.out.println(c.greet());
        System.out.println(((C)c).extra());
    }
}
"#);
    assert_eq!(out.trim(), "Hello from A\nHello from B\nHello from C\nHello from C via C");
}

#[test]
fn interface_static_method() {
    let out = run(r#"
interface MathOp {
    int apply(int a, int b);
    static MathOp add() { return (a, b) -> a + b; }
    static MathOp multiply() { return (a, b) -> a * b; }
    default MathOp andThen(MathOp next) {
        return (a, b) -> next.apply(this.apply(a, b), b);
    }
}
class Main {
    public static void main(String[] args) {
        MathOp add = MathOp.add();
        MathOp mul = MathOp.multiply();
        System.out.println(add.apply(3, 4));
        System.out.println(mul.apply(3, 4));
    }
}
"#);
    assert_eq!(out.trim(), "7\n12");
}

#[test]
fn annotations_basic() {
    let out = run(r#"
import java.lang.annotation.*;
@interface MyAnnotation {
    String value() default "default";
}
@MyAnnotation("hello")
class Greeter {
    @MyAnnotation("world")
    String greet() { return "hi"; }
}
class Main {
    public static void main(String[] args) {
        Greeter g = new Greeter();
        System.out.println(g.greet());
        System.out.println("done");
    }
}
"#);
    assert_eq!(out.trim(), "hi\ndone");
}

#[test]
fn regex_operations() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "hello world 123";
        System.out.println(s.matches(".*\\d+.*"));
        System.out.println(s.replaceAll("\\d+", "NUM"));
        System.out.println(s.replaceFirst("[a-z]+", "X"));
        String[] parts = "a,b,,c".split(",");
        System.out.println(parts.length);
        System.out.println(parts[0]);
        System.out.println(parts[1]);
    }
}
"#);
    assert_eq!(out.trim(), "true\nhello world NUM\nX world 123\n4\na\nb");
}

#[test]
fn java_time_basic() {
    let out = run(r#"
import java.time.*;
class Main {
    public static void main(String[] args) {
        LocalDate d = LocalDate.of(2024, 3, 15);
        System.out.println(d.getYear());
        System.out.println(d.getMonthValue());
        System.out.println(d.getDayOfMonth());
        LocalDate d2 = d.plusDays(10);
        System.out.println(d2.getDayOfMonth());
        System.out.println(d.isBefore(d2));
    }
}
"#);
    assert_eq!(out.trim(), "2024\n3\n15\n25\ntrue");
}

#[test]
fn stream_generate_and_iterate() {
    let out = run(r#"
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        // Stream.iterate: 0,1,2,3,4 — take first 5
        Stream.iterate(0, n -> n + 1)
            .limit(5)
            .forEach(n -> System.out.println(n));
        // Stream.generate: constant supplier, take 3
        int[] count = {0};
        Stream.generate(() -> 42)
            .limit(3)
            .forEach(n -> System.out.println(n));
    }
}
"#);
    assert_eq!(out.trim(), "0\n1\n2\n3\n4\n42\n42\n42");
}

#[test]
fn intstream_range() {
    let out = run(r#"
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        int sum = IntStream.range(1, 6).sum();
        System.out.println(sum);
        int sumClosed = IntStream.rangeClosed(1, 5).sum();
        System.out.println(sumClosed);
        IntStream.range(0, 3).forEach(i -> System.out.println(i));
    }
}
"#);
    assert_eq!(out.trim(), "15\n15\n0\n1\n2");
}

#[test]
fn linkedlist_as_deque() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        LinkedList<Integer> list = new LinkedList<>();
        list.add(1);
        list.add(2);
        list.add(3);
        System.out.println(list.size());
        System.out.println(list.get(0));
        list.addFirst(0);
        System.out.println(list.getFirst());
        list.addLast(4);
        System.out.println(list.getLast());
        list.removeFirst();
        System.out.println(list.size());
    }
}
"#);
    assert_eq!(out.trim(), "3\n1\n0\n4\n4");
}

#[test]
fn collections_addall_and_copy() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> list = new ArrayList<>();
        Collections.addAll(list, "a", "b", "c");
        System.out.println(list.size());
        System.out.println(list.get(1));
        Collections.sort(list, Collections.reverseOrder());
        System.out.println(list.get(0));
        List<String> copy = new ArrayList<>(list);
        Collections.reverse(copy);
        System.out.println(copy.get(0));
    }
}
"#);
    assert_eq!(out.trim(), "3\nb\nc\na");
}

#[test]
fn string_join_and_format() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("foo", "bar", "baz");
        String joined = String.join(", ", words);
        System.out.println(joined);
        System.out.println(String.join("-", "a", "b", "c"));
        System.out.printf("%.2f%n", 3.14159);
        System.out.printf("%05d%n", 42);
    }
}
"#);
    assert_eq!(out.trim(), "foo, bar, baz\na-b-c\n3.14\n00042");
}

#[test]
fn map_compute_and_merge() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> freq = new HashMap<>();
        String[] words = {"a", "b", "a", "c", "b", "a"};
        for (String w : words) {
            freq.merge(w, 1, (old, v) -> old + v);
        }
        System.out.println(freq.get("a"));
        System.out.println(freq.get("b"));
        System.out.println(freq.get("c"));
        freq.computeIfAbsent("d", k -> k.length());
        System.out.println(freq.get("d"));
    }
}
"#);
    assert_eq!(out.trim(), "3\n2\n1\n1");
}

#[test]
fn iterator_explicit() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> list = new ArrayList<>(Arrays.asList("x", "y", "z"));
        Iterator<String> it = list.iterator();
        while (it.hasNext()) {
            System.out.println(it.next());
        }
    }
}
"#);
    assert_eq!(out.trim(), "x\ny\nz");
}

#[test]
fn collections_frequency_and_disjoint() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> list = Arrays.asList("a", "b", "a", "c", "a");
        System.out.println(Collections.frequency(list, "a"));
        List<String> a = Arrays.asList("1", "2", "3");
        List<String> b = Arrays.asList("4", "5", "6");
        List<String> c = Arrays.asList("3", "4");
        System.out.println(Collections.disjoint(a, b));
        System.out.println(Collections.disjoint(a, c));
    }
}
"#);
    assert_eq!(out.trim(), "3\ntrue\nfalse");
}

#[test]
fn map_foreach_and_replaceall() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> map = new TreeMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.put("c", 3);
        map.forEach((k, v) -> System.out.println(k + "=" + v));
        map.replaceAll((k, v) -> v * 10);
        System.out.println(map.get("b"));
    }
}
"#);
    assert_eq!(out.trim(), "a=1\nb=2\nc=3\n20");
}

#[test]
fn string_strip_and_repeat() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "  hello  ";
        System.out.println(s.strip());
        System.out.println(s.stripLeading());
        System.out.println(s.stripTrailing());
        System.out.println("ab".repeat(3));
        System.out.println("".isEmpty());
        System.out.println("x".isEmpty());
    }
}
"#);
    assert_eq!(out.trim(), "hello\nhello  \n  hello\nababab\ntrue\nfalse");
}

#[test]
fn nested_class_static() {
    let out = run(r#"
class Outer {
    static int x = 10;
    static class Inner {
        int y;
        Inner(int y) { this.y = y; }
        int sum() { return Outer.x + y; }
    }
    public static void main(String[] args) {
        Inner i = new Inner(5);
        System.out.println(i.sum());
        Outer.x = 20;
        System.out.println(i.sum());
    }
}
"#);
    assert_eq!(out.trim(), "15\n25");
}

#[test]
fn ternary_and_conditional() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int x = 5;
        String s = x > 3 ? "big" : "small";
        System.out.println(s);
        int abs = x < 0 ? -x : x;
        System.out.println(abs);
        // nested ternary
        int y = 10;
        String cat = y < 5 ? "low" : y < 15 ? "mid" : "high";
        System.out.println(cat);
    }
}
"#);
    assert_eq!(out.trim(), "big\n5\nmid");
}

#[test]
fn bitwise_operations() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int a = 0b1010;  // 10
        int b = 0b1100;  // 12
        System.out.println(a & b);   // 8
        System.out.println(a | b);   // 14
        System.out.println(a ^ b);   // 6
        System.out.println(~a);      // -11
        System.out.println(a << 1);  // 20
        System.out.println(a >> 1);  // 5
        System.out.println(-1 >>> 28); // 15
    }
}
"#);
    assert_eq!(out.trim(), "8\n14\n6\n-11\n20\n5\n15");
}

#[test]
fn string_builder_chaining() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String result = new StringBuilder()
            .append("Hello")
            .append(", ")
            .append("World")
            .append("!")
            .toString();
        System.out.println(result);
        StringBuilder sb = new StringBuilder("abc");
        sb.insert(1, "X");
        System.out.println(sb.toString());
        sb.reverse();
        System.out.println(sb.toString());
        System.out.println(sb.length());
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!\naXbc\ncbXa\n4");
}

#[test]
fn array_operations() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    public static void main(String[] args) {
        int[] arr = {5, 2, 8, 1, 9, 3};
        Arrays.sort(arr);
        System.out.println(Arrays.toString(arr));
        int idx = Arrays.binarySearch(arr, 8);
        System.out.println(idx);
        int[] copy = Arrays.copyOf(arr, 4);
        System.out.println(Arrays.toString(copy));
        int[] range = Arrays.copyOfRange(arr, 2, 5);
        System.out.println(Arrays.toString(range));
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 5, 8, 9]\n4\n[1, 2, 3, 5]\n[3, 5, 8]");
}

#[test]
fn generic_bounded_type() {
    let out = run(r#"
class Box<T extends Comparable<T>> {
    private T value;
    Box(T value) { this.value = value; }
    T getValue() { return value; }
    boolean isGreaterThan(Box<T> other) {
        return value.compareTo(other.getValue()) > 0;
    }
}
class Main {
    public static void main(String[] args) {
        Box<Integer> a = new Box<>(10);
        Box<Integer> b = new Box<>(5);
        System.out.println(a.getValue());
        System.out.println(a.isGreaterThan(b));
        System.out.println(b.isGreaterThan(a));
    }
}
"#);
    assert_eq!(out.trim(), "10\ntrue\nfalse");
}

#[test]
fn interface_default_method() {
    let out = run(r#"
interface Greeter {
    String name();
    default String greet() {
        return "Hello, " + name() + "!";
    }
}
class Person implements Greeter {
    private String n;
    Person(String n) { this.n = n; }
    public String name() { return n; }
}
class Main {
    public static void main(String[] args) {
        Person p = new Person("Alice");
        System.out.println(p.greet());
        System.out.println(p.name());
    }
}
"#);
    assert_eq!(out.trim(), "Hello, Alice!\nAlice");
}

#[test]
fn exception_message_and_type() {
    let out = run(r#"
class Main {
    static int divide(int a, int b) {
        if (b == 0) throw new ArithmeticException("division by zero");
        return a / b;
    }
    public static void main(String[] args) {
        try {
            System.out.println(divide(10, 2));
            System.out.println(divide(10, 0));
        } catch (ArithmeticException e) {
            System.out.println("caught: " + e.getMessage());
        }
        try {
            String s = null;
            s.length();
        } catch (NullPointerException e) {
            System.out.println("npe caught");
        }
    }
}
"#);
    assert_eq!(out.trim(), "5\ncaught: division by zero\nnpe caught");
}

#[test]
fn map_of_and_list_of() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<Integer> list = List.of(1, 2, 3, 4, 5);
        System.out.println(list.size());
        System.out.println(list.get(2));
        Set<String> set = Set.of("a", "b", "c");
        System.out.println(set.size());
        System.out.println(set.contains("b"));
    }
}
"#);
    assert_eq!(out.trim(), "5\n3\n3\ntrue");
}

#[test]
fn type_casting_and_autoboxing() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // Narrowing cast
        double pi = 3.14;
        int truncated = (int) pi;
        System.out.println(truncated);
        // Autoboxing
        Integer boxed = 100;
        int unboxed = boxed;
        System.out.println(unboxed);
        // String conversion
        String s = String.valueOf(42);
        System.out.println(s.length());
        int parsed = Integer.parseInt("123");
        System.out.println(parsed + 1);
        // double literal
        double d = 3.0;
        System.out.println(d);
    }
}
"#);
    assert_eq!(out.trim(), "3\n100\n2\n124\n3.0");
}

#[test]
fn while_and_break_continue() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int sum = 0;
        int i = 0;
        while (i < 10) {
            i++;
            if (i % 2 == 0) continue;
            if (i > 7) break;
            sum += i;
        }
        System.out.println(sum); // 1+3+5+7 = 16
        // do-while
        int n = 5;
        int fact = 1;
        do {
            fact *= n;
            n--;
        } while (n > 0);
        System.out.println(fact); // 120
    }
}
"#);
    assert_eq!(out.trim(), "16\n120");
}

#[test]
fn string_number_conversions() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Integer.parseInt("42"));
        System.out.println(Double.parseDouble("3.14"));
        System.out.println(Integer.toString(255, 16));
        System.out.println(Integer.toBinaryString(10));
        System.out.println(Integer.toHexString(255));
        System.out.println(Integer.toOctalString(8));
        System.out.println(Integer.max(3, 7));
        System.out.println(Integer.min(3, 7));
        System.out.println(Integer.sum(3, 7));
    }
}
"#);
    assert_eq!(out.trim(), "42\n3.14\nff\n1010\nff\n10\n7\n3\n10");
}

#[test]
fn abstract_class_template_method() {
    let out = run(r#"
abstract class Shape {
    abstract double area();
    String describe() {
        return "Shape with area " + area();
    }
}
class Circle extends Shape {
    double r;
    Circle(double r) { this.r = r; }
    double area() { return Math.PI * r * r; }
}
class Rectangle extends Shape {
    double w, h;
    Rectangle(double w, double h) { this.w = w; this.h = h; }
    double area() { return w * h; }
}
class Main {
    public static void main(String[] args) {
        Shape c = new Circle(1.0);
        Shape r = new Rectangle(3.0, 4.0);
        System.out.printf("%.4f%n", c.area());
        System.out.println(r.area());
        System.out.println(r.describe());
    }
}
"#);
    assert_eq!(out.trim(), "3.1416\n12.0\nShape with area 12.0");
}

#[test]
fn varargs_and_overloading() {
    let out = run(r#"
class Calc {
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }
    static String join(String sep, String... parts) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < parts.length; i++) {
            if (i > 0) sb.append(sep);
            sb.append(parts[i]);
        }
        return sb.toString();
    }
}
class Main {
    public static void main(String[] args) {
        System.out.println(Calc.sum(1, 2, 3));
        System.out.println(Calc.sum(10, 20));
        System.out.println(Calc.join(", ", "a", "b", "c"));
        System.out.println(Calc.join("-", "x", "y"));
    }
}
"#);
    assert_eq!(out.trim(), "6\n30\na, b, c\nx-y");
}

#[test]
fn static_factory_pattern() {
    let out = run(r#"
class Color {
    private int r, g, b;
    private Color(int r, int g, int b) { this.r = r; this.g = g; this.b = b; }
    static Color of(int r, int g, int b) { return new Color(r, g, b); }
    static Color red()   { return new Color(255, 0, 0); }
    static Color green() { return new Color(0, 255, 0); }
    public String toString() { return "(" + r + "," + g + "," + b + ")"; }
}
class Main {
    public static void main(String[] args) {
        Color c = Color.of(100, 150, 200);
        System.out.println(c);
        System.out.println(Color.red());
        System.out.println(Color.green());
    }
}
"#);
    assert_eq!(out.trim(), "(100,150,200)\n(255,0,0)\n(0,255,0)");
}

#[test]
fn sealed_interface_simulation() {
    let out = run(r#"
interface Expr {}
class Num implements Expr {
    int val;
    Num(int val) { this.val = val; }
}
class Add implements Expr {
    Expr left, right;
    Add(Expr left, Expr right) { this.left = left; this.right = right; }
}
class Mul implements Expr {
    Expr left, right;
    Mul(Expr left, Expr right) { this.left = left; this.right = right; }
}
class Eval {
    static int eval(Expr e) {
        if (e instanceof Num n) return n.val;
        if (e instanceof Add a) return eval(a.left) + eval(a.right);
        if (e instanceof Mul m) return eval(m.left) * eval(m.right);
        return 0;
    }
}
class Main {
    public static void main(String[] args) {
        // (2 + 3) * 4
        Expr e = new Mul(new Add(new Num(2), new Num(3)), new Num(4));
        System.out.println(Eval.eval(e));
        // 1 + 2 + 3
        Expr e2 = new Add(new Add(new Num(1), new Num(2)), new Num(3));
        System.out.println(Eval.eval(e2));
    }
}
"#);
    assert_eq!(out.trim(), "20\n6");
}

#[test]
fn string_tokenizer_split() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        String csv = "name,age,city";
        String[] parts = csv.split(",");
        for (String p : parts) System.out.println(p);
        // split with limit
        String s = "a:b:c:d";
        String[] limited = s.split(":", 2);
        System.out.println(limited.length);
        System.out.println(limited[0]);
        System.out.println(limited[1]);
    }
}
"#);
    assert_eq!(out.trim(), "name\nage\ncity\n2\na\nb:c:d");
}

#[test]
fn math_advanced() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Math.abs(-5));
        System.out.println(Math.abs(-3.14));
        System.out.printf("%.4f%n", Math.sqrt(2.0));
        System.out.printf("%.4f%n", Math.log(Math.E));
        System.out.printf("%.4f%n", Math.sin(Math.PI / 2));
        System.out.println(Math.pow(2, 10));
        System.out.println(Math.max(3, 7));
        System.out.println(Math.min(3, 7));
        System.out.println(Math.floor(3.9));
        System.out.println(Math.ceil(3.1));
        System.out.println(Math.round(3.5));
    }
}
"#);
    assert_eq!(out.trim(), "5\n3.14\n1.4142\n1.0000\n1.0000\n1024.0\n7\n3\n3.0\n4.0\n4");
}

#[test]
fn comparable_natural_order() {
    let out = run(r#"
import java.util.*;
class Student implements Comparable<Student> {
    String name;
    int grade;
    Student(String name, int grade) { this.name = name; this.grade = grade; }
    public int compareTo(Student other) { return Integer.compare(this.grade, other.grade); }
    public String toString() { return name + ":" + grade; }
}
class Main {
    public static void main(String[] args) {
        List<Student> students = new ArrayList<>();
        students.add(new Student("Alice", 85));
        students.add(new Student("Bob", 92));
        students.add(new Student("Charlie", 78));
        Collections.sort(students);
        for (Student s : students) System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "Charlie:78\nAlice:85\nBob:92");
}

#[test]
fn stack_operations() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Deque<Integer> stack = new ArrayDeque<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        System.out.println(stack.peek());
        System.out.println(stack.pop());
        System.out.println(stack.size());
        stack.push(10);
        while (!stack.isEmpty()) {
            System.out.println(stack.pop());
        }
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n2\n10\n2\n1");
}

#[test]
fn optional_chaining() {
    let out = run(r#"
import java.util.*;
class Main {
    static Optional<String> findName(boolean found) {
        return found ? Optional.of("Alice") : Optional.empty();
    }
    public static void main(String[] args) {
        Optional<String> name = findName(true);
        System.out.println(name.isPresent());
        System.out.println(name.get());
        Optional<String> empty = findName(false);
        System.out.println(empty.isPresent());
        System.out.println(empty.orElse("default"));
        System.out.println(name.map(s -> s.toUpperCase()).orElse("none"));
    }
}
"#);
    assert_eq!(out.trim(), "true\nAlice\nfalse\ndefault\nALICE");
}

#[test]
fn multiline_string_and_char_ops() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        char c = 'A';
        System.out.println(c);
        System.out.println((int) c);
        System.out.println(Character.isLetter(c));
        System.out.println(Character.isDigit(c));
        System.out.println(Character.toLowerCase(c));
        System.out.println(Character.toUpperCase('z'));
        System.out.println(Character.isWhitespace(' '));
    }
}
"#);
    assert_eq!(out.trim(), "A\n65\ntrue\nfalse\na\nZ\ntrue");
}

#[test]
fn two_dimensional_list() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<List<Integer>> matrix = new ArrayList<>();
        for (int i = 0; i < 3; i++) {
            List<Integer> row = new ArrayList<>();
            for (int j = 0; j < 3; j++) {
                row.add(i * 3 + j + 1);
            }
            matrix.add(row);
        }
        for (List<Integer> row : matrix) {
            for (int j = 0; j < row.size(); j++) {
                if (j > 0) System.out.print(" ");
                System.out.print(row.get(j));
            }
            System.out.println();
        }
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3\n4 5 6\n7 8 9");
}

#[test]
fn stream_collectors_grouping() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("apple", "banana", "avocado", "blueberry", "cherry");
        // group by first letter
        Map<String, List<String>> grouped = words.stream()
            .collect(Collectors.groupingBy(w -> w.substring(0, 1)));
        // print sorted keys
        new TreeMap<>(grouped).forEach((k, v) -> {
            Collections.sort(v);
            System.out.println(k + ": " + v.size());
        });
    }
}
"#);
    assert_eq!(out.trim(), "a: 2\nb: 2\nc: 1");
}

#[test]
fn string_char_operations() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello, World!";
        // charAt returns char
        char c = s.charAt(0);
        System.out.println(c);
        // toCharArray
        char[] arr = s.toCharArray();
        System.out.println(arr.length);
        // String from chars
        StringBuilder sb = new StringBuilder();
        for (char ch : arr) {
            if (Character.isUpperCase(ch)) sb.append(ch);
        }
        System.out.println(sb.toString());
        // indexOf char
        System.out.println(s.indexOf('o'));
        System.out.println(s.lastIndexOf('o'));
    }
}
"#);
    assert_eq!(out.trim(), "H\n13\nHW\n4\n8");
}

#[test]
fn number_formatting() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.printf("%d%n", 42);
        System.out.printf("%05d%n", 42);
        System.out.printf("%-10s|%n", "left");
        System.out.printf("%10s|%n", "right");
        System.out.printf("%.3f%n", Math.PI);
        System.out.printf("%e%n", 123456.789);
        System.out.printf("%b%n", true);
    }
}
"#);
    assert_eq!(out.trim(), "42\n00042\nleft      |\n     right|\n3.142\n1.234568e+05\ntrue");
}

#[test]
fn functional_composition() {
    let out = run(r#"
import java.util.*;
import java.util.function.*;
class Main {
    public static void main(String[] args) {
        Function<Integer, Integer> doubler = x -> x * 2;
        Function<Integer, Integer> adder = x -> x + 3;
        // andThen
        Function<Integer, Integer> doubleThenAdd = doubler.andThen(adder);
        System.out.println(doubleThenAdd.apply(5));  // 5*2+3=13
        // compose
        Function<Integer, Integer> addThenDouble = doubler.compose(adder);
        System.out.println(addThenDouble.apply(5));  // (5+3)*2=16
        // Predicate
        Predicate<Integer> isEven = n -> n % 2 == 0;
        Predicate<Integer> isPositive = n -> n > 0;
        System.out.println(isEven.test(4));
        System.out.println(isEven.and(isPositive).test(4));
        System.out.println(isEven.and(isPositive).test(-4));
    }
}
"#);
    assert_eq!(out.trim(), "13\n16\ntrue\ntrue\nfalse");
}

#[test]
fn enum_with_fields() {
    let out = run(r#"
class Main {
    enum Planet {
        MERCURY(3.303e+23, 2.4397e6),
        VENUS(4.869e+24, 6.0518e6),
        EARTH(5.976e+24, 6.37814e6);
        private double mass;
        private double radius;
        Planet(double mass, double radius) {
            this.mass = mass;
            this.radius = radius;
        }
        double surfaceGravity() {
            double G = 6.67300E-11;
            return G * mass / (radius * radius);
        }
    }
    public static void main(String[] args) {
        System.out.println(Planet.EARTH.surfaceGravity() > 9.0);
        System.out.println(Planet.MERCURY.surfaceGravity() < 5.0);
    }
}
"#);
    assert_eq!(out.trim(), "true\ntrue");
}

#[test]
fn iterator_pattern() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> list = new ArrayList<>(Arrays.asList("a", "b", "c", "d"));
        // remove while iterating using iterator
        Iterator<String> it = list.iterator();
        while (it.hasNext()) {
            String s = it.next();
            if (s.equals("b") || s.equals("d")) it.remove();
        }
        System.out.println(list.size());
        for (String s : list) System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "2\na\nc");
}

#[test]
fn string_format_advanced() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // %05d zero-padded
        System.out.println(String.format("%05d", 42));
        // %-10s left-aligned
        System.out.println(String.format("%-10s|", "hi"));
        // %+d signed
        System.out.printf("%+d%n", 42);
        System.out.printf("%+d%n", -42);
        // %10.2f
        System.out.println(String.format("%10.2f", 3.14159));
    }
}
"#);
    assert_eq!(out.trim(), "00042\nhi        |\n+42\n-42\n      3.14");
}

#[test]
fn multi_catch_exception() {
    let out = run(r#"
class Main {
    static int parse(String s) {
        return Integer.parseInt(s);
    }
    static int index(int[] arr, int i) {
        return arr[i];
    }
    public static void main(String[] args) {
        try {
            parse("abc");
        } catch (NumberFormatException e) {
            System.out.println("NFE: " + e.getMessage());
        }
        try {
            int[] arr = {1, 2, 3};
            index(arr, 10);
        } catch (ArrayIndexOutOfBoundsException e) {
            System.out.println("AIOOBE");
        }
        try {
            String s = null;
            s.length();
        } catch (NullPointerException e) {
            System.out.println("NPE");
        }
        System.out.println("done");
    }
}
"#);
    assert_eq!(out.trim(), "NFE: For input string: \"abc\"\nAIOOBE\nNPE\ndone");
}

#[test]
fn generic_stack() {
    let out = run(r#"
import java.util.*;
class MyStack<T> {
    private List<T> data = new ArrayList<>();
    public void push(T item) { data.add(item); }
    public T pop() {
        if (data.isEmpty()) throw new RuntimeException("empty");
        return data.remove(data.size() - 1);
    }
    public T peek() { return data.get(data.size() - 1); }
    public boolean isEmpty() { return data.isEmpty(); }
    public int size() { return data.size(); }
}
class Main {
    public static void main(String[] args) {
        MyStack<Integer> s = new MyStack<>();
        s.push(1); s.push(2); s.push(3);
        System.out.println(s.size());
        System.out.println(s.peek());
        System.out.println(s.pop());
        System.out.println(s.size());
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n3\n2");
}

#[test]
fn lambda_sort_and_stream() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> words = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "date"));
        // sort by length then alphabetically
        words.sort(Comparator.comparingInt(String::length).thenComparing(Comparator.naturalOrder()));
        words.forEach(System.out::println);
    }
}
"#);
    assert_eq!(out.trim(), "date\napple\nbanana\ncherry");
}

#[test]
fn switch_yield_in_block() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int x = 5;
        String result = switch (x) {
            case 1, 2 -> {
                int temp = x * 2;
                yield "small:" + temp;
            }
            case 3, 4, 5 -> {
                try {
                    int y = 10 / (x - 5);
                    yield "medium";
                } catch (ArithmeticException e) {
                    yield "error";
                }
            }
            default -> "large";
        };
        System.out.println(result);
    }
}
"#);
    assert_eq!(out.trim(), "error");
}

#[test]
fn record_compact_constructor() {
    let out = run(r#"
record Range(int lo, int hi) {
    Range {
        if (lo > hi) throw new IllegalArgumentException("bad range");
    }
    int size() { return hi - lo; }
}
class Main {
    public static void main(String[] args) {
        Range r = new Range(2, 7);
        System.out.println(r.lo());
        System.out.println(r.hi());
        System.out.println(r.size());
        try {
            Range bad = new Range(9, 1);
            System.out.println("no throw");
        } catch (IllegalArgumentException e) {
            System.out.println("caught");
        }
    }
}
"#);
    assert_eq!(out.trim(), "2\n7\n5\ncaught");
}

#[test]
fn interface_default_method_conflict() {
    let out = run(r#"
interface Greeter {
    default String greet() { return "Hello"; }
}
interface Farewell {
    default String greet() { return "Goodbye"; }
}
class Bilingual implements Greeter, Farewell {
    public String greet() { return "Hi and Bye"; }
}
class Main {
    public static void main(String[] args) {
        Bilingual b = new Bilingual();
        System.out.println(b.greet());
        Greeter g = b;
        System.out.println(g.greet());
        Farewell f = b;
        System.out.println(f.greet());
    }
}
"#);
    assert_eq!(out.trim(), "Hi and Bye\nHi and Bye\nHi and Bye");
}

#[test]
fn unmodifiable_list() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> original = new ArrayList<>();
        original.add("a");
        original.add("b");
        List<String> unmod = Collections.unmodifiableList(original);
        System.out.println(unmod.size());
        System.out.println(unmod.get(0));
        try {
            unmod.add("c");
            System.out.println("ERROR");
        } catch (UnsupportedOperationException e) {
            System.out.println("add blocked");
        }
        try {
            unmod.remove(0);
            System.out.println("ERROR");
        } catch (UnsupportedOperationException e) {
            System.out.println("remove blocked");
        }
    }
}
"#);
    assert_eq!(out.trim(), "2\na\nadd blocked\nremove blocked");
}

#[test]
fn nested_lambda_loop_capture() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<Integer> results = new ArrayList<>();
        for (int i = 1; i <= 3; i++) {
            final int fi = i;
            List<Integer> nums = Arrays.asList(fi, fi + 1, fi + 2);
            nums.stream().map(n -> n * fi).forEach(results::add);
        }
        for (int r : results) System.out.println(r);
    }
}
"#);
    assert_eq!(out.trim(), "1\n2\n3\n4\n6\n8\n9\n12\n15");
}

#[test]
fn nested_try_catch_finally() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        try {
            try {
                int x = 10 / 0;
            } catch (ArithmeticException e) {
                System.out.println("inner catch");
            } finally {
                System.out.println("inner finally");
            }
            System.out.println("after inner");
        } catch (RuntimeException e) {
            System.out.println("outer catch");
        } finally {
            System.out.println("outer finally");
        }
    }
}
"#);
    assert_eq!(out.trim(), "inner catch\ninner finally\nafter inner\nouter finally");
}

#[test]
fn static_method_ref_stream() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> nums = Arrays.asList("3", "1", "2");
        List<Integer> parsed = nums.stream()
            .map(Integer::parseInt)
            .collect(Collectors.toList());
        parsed.sort(null);
        for (int n : parsed) System.out.println(n);

        List<Integer> values = Arrays.asList(10, 20, 30);
        List<String> strs = values.stream()
            .map(String::valueOf)
            .collect(Collectors.toList());
        for (String s : strs) System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "1\n2\n3\n10\n20\n30");
}

#[test]
fn multi_field_sort() {
    let out = run(r#"
import java.util.*;
class Item {
    int priority;
    String name;
    Item(int p, String n) { this.priority = p; this.name = n; }
    public String toString() { return name + ":" + priority; }
}
class Main {
    public static void main(String[] args) {
        List<Item> items = new ArrayList<>();
        items.add(new Item(2, "b"));
        items.add(new Item(1, "c"));
        items.add(new Item(1, "a"));
        items.add(new Item(2, "a"));
        items.sort((x, y) -> {
            int cmp = Integer.compare(x.priority, y.priority);
            return cmp != 0 ? cmp : x.name.compareTo(y.name);
        });
        for (Item i : items) System.out.println(i);
    }
}
"#);
    assert_eq!(out.trim(), "a:1\nc:1\na:2\nb:2");
}

#[test]
fn recursive_generic_tree() {
    let out = run(r#"
import java.util.*;
class TreeNode {
    int value;
    List<TreeNode> children;
    TreeNode(int v) { this.value = v; this.children = new ArrayList<>(); }
    void add(TreeNode child) { children.add(child); }
    int countNodes() {
        int count = 1;
        for (TreeNode child : children) count += child.countNodes();
        return count;
    }
    int sumValues() {
        int sum = value;
        for (TreeNode child : children) sum += child.sumValues();
        return sum;
    }
}
class Main {
    public static void main(String[] args) {
        TreeNode root = new TreeNode(1);
        TreeNode left = new TreeNode(2);
        TreeNode right = new TreeNode(3);
        root.add(left);
        root.add(right);
        left.add(new TreeNode(4));
        System.out.println(root.countNodes());
        System.out.println(root.sumValues());
    }
}
"#);
    assert_eq!(out.trim(), "4\n10");
}

#[test]
fn exception_cause_chain() {
    let out = run(r#"
class Main {
    static void level3() throws Exception {
        throw new Exception("root cause");
    }
    static void level2() throws Exception {
        try { level3(); }
        catch (Exception e) { throw new RuntimeException("level2 error", e); }
    }
    static void level1() throws Exception {
        try { level2(); }
        catch (Exception e) { throw new RuntimeException("level1 error", e); }
    }
    public static void main(String[] args) {
        try {
            level1();
        } catch (Exception e) {
            System.out.println(e.getMessage());
            Throwable cause = e.getCause();
            System.out.println(cause.getMessage());
            Throwable root = cause.getCause();
            System.out.println(root.getMessage());
        }
    }
}
"#);
    assert_eq!(out.trim(), "level1 error\nlevel2 error\nroot cause");
}

#[test]
fn arrays_fill_and_copyof() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    public static void main(String[] args) {
        int[] a = new int[5];
        Arrays.fill(a, 7);
        for (int x : a) System.out.print(x + " ");
        System.out.println();

        int[] b = Arrays.copyOf(a, 3);
        for (int x : b) System.out.print(x + " ");
        System.out.println();

        int[] c = {5, 3, 1, 4, 2};
        Arrays.sort(c);
        for (int x : c) System.out.print(x + " ");
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "7 7 7 7 7 \n7 7 7 \n1 2 3 4 5");
}

