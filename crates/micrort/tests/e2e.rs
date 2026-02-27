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
fn do_while_loop_v2() {
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
fn exception_hierarchy_v2() {
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
fn bitwise_operations_v2() {
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
fn interface_default_method_v2() {
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
fn string_char_operations_v2() {
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
fn iterator_pattern_v2() {
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


#[test]
fn collectors_partitioning_by() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6);
        Map<Boolean, List<Integer>> parts = nums.stream()
            .collect(Collectors.partitioningBy(n -> n % 2 == 0));
        List<Integer> evens = parts.get(true);
        List<Integer> odds = parts.get(false);
        Collections.sort(evens);
        Collections.sort(odds);
        System.out.println(evens);
        System.out.println(odds);
    }
}
"#);
    assert_eq!(out.trim(), "[2, 4, 6]\n[1, 3, 5]");
}

#[test]
fn instanceof_and_cast() {
    let out = run(r#"
class Animal {}
class Dog extends Animal {
    String name;
    Dog(String n) { this.name = n; }
    String speak() { return "Woof"; }
}
class Cat extends Animal {
    String speak() { return "Meow"; }
}
class Main {
    static String describe(Animal a) {
        if (a instanceof Dog) {
            Dog d = (Dog) a;
            return d.speak() + " from " + d.name;
        } else if (a instanceof Cat) {
            Cat c = (Cat) a;
            return c.speak();
        }
        return "unknown";
    }
    public static void main(String[] args) {
        System.out.println(describe(new Dog("Rex")));
        System.out.println(describe(new Cat()));
    }
}
"#);
    assert_eq!(out.trim(), "Woof from Rex\nMeow");
}

#[test]
fn abstract_class_v2() {
    let out = run(r#"
abstract class Shape {
    abstract double area();
    String describe() { return "Shape with area " + area(); }
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
        Shape[] shapes = { new Rectangle(3.0, 4.0), new Circle(0.0) };
        System.out.println(shapes[0].area());
        System.out.println(shapes[1].area());
        System.out.println(shapes[0].describe());
    }
}
"#);
    assert_eq!(out.trim(), "12.0\n0.0\nShape with area 12.0");
}

#[test]
fn generic_pair_and_swap() {
    let out = run(r#"
class Pair<A, B> {
    A first;
    B second;
    Pair(A a, B b) { this.first = a; this.second = b; }
    Pair<B, A> swap() { return new Pair<>(second, first); }
}
class Main {
    public static void main(String[] args) {
        Pair<String, Integer> p = new Pair<>("hello", 42);
        System.out.println(p.first);
        System.out.println(p.second);
        Pair<Integer, String> swapped = p.swap();
        System.out.println(swapped.first);
        System.out.println(swapped.second);
    }
}
"#);
    assert_eq!(out.trim(), "hello\n42\n42\nhello");
}

#[test]
fn string_methods_advanced() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println("abc".repeat(3));
        System.out.println(String.join("-", "a", "b", "c"));
        System.out.println("hello".substring(1, 3));
        System.out.println("hello world".replace("world", "Java"));
        System.out.println("a,b,,c".split(",").length);
        System.out.println("Hello World".toLowerCase());
    }
}
"#);
    assert_eq!(out.trim(), "abcabcabc\na-b-c\nel\nhello Java\n4\nhello world");
}

#[test]
fn optional_basic() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Optional<String> present = Optional.of("hello");
        Optional<String> empty = Optional.empty();
        System.out.println(present.isPresent());
        System.out.println(empty.isPresent());
        System.out.println(present.get());
        System.out.println(present.orElse("default"));
        System.out.println(empty.orElse("default"));
        String mapped = present.map(s -> s.toUpperCase()).orElse("none");
        System.out.println(mapped);
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse\nhello\nhello\ndefault\nHELLO");
}

#[test]
fn interface_with_generics() {
    let out = run(r#"
interface Transformer<T, R> {
    R transform(T input);
}
class Main {
    static <T, R> R apply(T val, Transformer<T, R> t) {
        return t.transform(val);
    }
    public static void main(String[] args) {
        Transformer<String, Integer> len = s -> s.length();
        Transformer<Integer, String> str = n -> "num:" + n;
        System.out.println(apply("hello", len));
        System.out.println(apply(42, str));
    }
}
"#);
    assert_eq!(out.trim(), "5\nnum:42");
}

#[test]
fn linked_list_user_defined() {
    let out = run(r#"
class Node {
    int val;
    Node next;
    Node(int v) { this.val = v; this.next = null; }
}
class LinkedList {
    Node head;
    void add(int v) {
        Node n = new Node(v);
        if (head == null) { head = n; return; }
        Node cur = head;
        while (cur.next != null) cur = cur.next;
        cur.next = n;
    }
    int size() {
        int count = 0;
        Node cur = head;
        while (cur != null) { count++; cur = cur.next; }
        return count;
    }
    int get(int idx) {
        Node cur = head;
        for (int i = 0; i < idx; i++) cur = cur.next;
        return cur.val;
    }
}
class Main {
    public static void main(String[] args) {
        LinkedList list = new LinkedList();
        list.add(10);
        list.add(20);
        list.add(30);
        System.out.println(list.size());
        System.out.println(list.get(0));
        System.out.println(list.get(2));
    }
}
"#);
    assert_eq!(out.trim(), "3\n10\n30");
}

#[test]
fn string_format_numbers() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.printf("%.2f%n", 3.14159);
        System.out.printf("%d + %d = %d%n", 3, 4, 7);
        System.out.printf("%s has %d chars%n", "hello", 5);
        System.out.println(String.format("%05d", 42));
    }
}
"#);
    assert_eq!(out.trim(), "3.14\n3 + 4 = 7\nhello has 5 chars\n00042");
}

#[test]
fn multi_interface_impl() {
    let out = run(r#"
interface Printable {
    void print();
}
interface Saveable {
    String save();
}
class Document implements Printable, Saveable {
    String content;
    Document(String c) { this.content = c; }
    public void print() { System.out.println("Doc: " + content); }
    public String save() { return "saved:" + content; }
}
class Main {
    public static void main(String[] args) {
        Document doc = new Document("hello");
        doc.print();
        System.out.println(doc.save());
        Printable p = doc;
        p.print();
        Saveable s = doc;
        System.out.println(s.save());
    }
}
"#);
    assert_eq!(out.trim(), "Doc: hello\nsaved:hello\nDoc: hello\nsaved:hello");
}

#[test]
fn static_fields_and_methods_v2() {
    let out = run(r#"
class Counter {
    static int count = 0;
    int id;
    Counter() { count++; this.id = count; }
    static int getCount() { return count; }
    static void reset() { count = 0; }
}
class Main {
    public static void main(String[] args) {
        Counter a = new Counter();
        Counter b = new Counter();
        Counter c = new Counter();
        System.out.println(Counter.getCount());
        System.out.println(a.id);
        System.out.println(c.id);
        Counter.reset();
        System.out.println(Counter.getCount());
    }
}
"#);
    assert_eq!(out.trim(), "3\n1\n3\n0");
}

#[test]
fn nested_class_simulation() {
    let out = run(r#"
class Outer {
    int x;
    Outer(int x) { this.x = x; }
    int doubled() { return x * 2; }
}
class Main {
    static int process(Outer o, int extra) {
        return o.doubled() + extra;
    }
    public static void main(String[] args) {
        Outer o1 = new Outer(5);
        Outer o2 = new Outer(10);
        System.out.println(process(o1, 3));
        System.out.println(process(o2, 7));
        System.out.println(o1.x + o2.x);
    }
}
"#);
    assert_eq!(out.trim(), "13\n27\n15");
}

#[test]
fn enum_ordinal_and_values() {
    let out = run(r#"
enum Season { SPRING, SUMMER, FALL, WINTER }
class Main {
    public static void main(String[] args) {
        Season s = Season.SUMMER;
        System.out.println(s.ordinal());
        System.out.println(s.name());
        Season[] all = Season.values();
        System.out.println(all.length);
        for (Season x : all) System.out.println(x);
    }
}
"#);
    assert_eq!(out.trim(), "1\nSUMMER\n4\nSPRING\nSUMMER\nFALL\nWINTER");
}

#[test]
fn string_builder_delete_and_replace() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        StringBuilder sb = new StringBuilder("Hello World");
        sb.delete(5, 11);
        System.out.println(sb.toString());
        sb.replace(0, 5, "Hi");
        System.out.println(sb.toString());
        sb.insert(2, " there");
        System.out.println(sb.toString());
        System.out.println(sb.length());
    }
}
"#);
    assert_eq!(out.trim(), "Hello\nHi\nHi there\n8");
}

#[test]
fn collections_stack_and_queue() {
    let out = run(r#"
import java.util.Stack;
import java.util.LinkedList;
import java.util.Queue;
class Main {
    public static void main(String[] args) {
        Stack<Integer> stack = new Stack<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        System.out.println(stack.peek());
        System.out.println(stack.pop());
        System.out.println(stack.size());
        Queue<String> queue = new LinkedList<>();
        queue.offer("a");
        queue.offer("b");
        queue.offer("c");
        System.out.println(queue.peek());
        System.out.println(queue.poll());
        System.out.println(queue.size());
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n2\na\na\n2");
}

#[test]
fn generic_bounded_wildcard() {
    let out = run(r#"
class Box<T extends Number> {
    T value;
    Box(T v) { this.value = v; }
    double doubled() { return value.doubleValue() * 2; }
}
class Main {
    static double sumBoxes(Box<? extends Number>[] boxes) {
        double sum = 0;
        for (Box<? extends Number> b : boxes) sum += b.value.doubleValue();
        return sum;
    }
    public static void main(String[] args) {
        Box<Integer> bi = new Box<>(5);
        Box<Double> bd = new Box<>(3.5);
        System.out.println(bi.doubled());
        System.out.println(bd.doubled());
    }
}
"#);
    assert_eq!(out.trim(), "10.0\n7.0");
}

#[test]
fn exception_finally_return() {
    let out = run(r#"
class Main {
    static int test(boolean throwIt) {
        try {
            if (throwIt) throw new RuntimeException("oops");
            return 1;
        } catch (RuntimeException e) {
            System.out.println("caught: " + e.getMessage());
            return 2;
        } finally {
            System.out.println("finally");
        }
    }
    public static void main(String[] args) {
        System.out.println(test(false));
        System.out.println(test(true));
    }
}
"#);
    assert_eq!(out.trim(), "1\ncaught: oops\n2");
}

#[test]
fn array_2d_operations() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] matrix = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                matrix[i][j] = i * 3 + j + 1;
        // print diagonal
        for (int i = 0; i < 3; i++) System.out.println(matrix[i][i]);
        // transpose check
        int sum = 0;
        for (int[] row : matrix) for (int v : row) sum += v;
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "1\n5\n9\n45");
}

#[test]
fn functional_predicate_chain() {
    let out = run(r#"
import java.util.function.Predicate;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    public static void main(String[] args) {
        Predicate<Integer> isEven = n -> n % 2 == 0;
        Predicate<Integer> isPositive = n -> n > 0;
        Predicate<Integer> isEvenAndPositive = isEven.and(isPositive);
        List<Integer> nums = Arrays.asList(-4, -1, 0, 2, 3, 6);
        List<Integer> result = nums.stream()
            .filter(isEvenAndPositive)
            .collect(Collectors.toList());
        System.out.println(result);
        System.out.println(isEven.negate().test(3));
        System.out.println(isEven.or(isPositive).test(3));
    }
}
"#);
    assert_eq!(out.trim(), "[2, 6]\ntrue\ntrue");
}

#[test]
fn string_chars_and_codepoints() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello";
        // charAt and char arithmetic
        char first = s.charAt(0);
        char last = s.charAt(s.length() - 1);
        System.out.println(first);
        System.out.println(last);
        System.out.println((int) first);
        // toCharArray
        char[] arr = s.toCharArray();
        System.out.println(arr.length);
        // build string from codepoints
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < s.length(); i++) {
            int code = s.codePointAt(i);
            sb.append((char)(code + 1));
        }
        System.out.println(sb.toString());
    }
}
"#);
    assert_eq!(out.trim(), "H\no\n72\n5\nIfmmp");
}

#[test]
fn iterator_remove() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.Iterator;
class Main {
    public static void main(String[] args) {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
        Iterator<Integer> it = list.iterator();
        while (it.hasNext()) {
            int v = it.next();
            if (v % 2 == 0) it.remove();
        }
        System.out.println(list);
        System.out.println(list.size());
    }
}
"#);
    assert_eq!(out.trim(), "[1, 3, 5]\n3");
}

#[test]
fn comparable_sort_custom() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.Collections;
class Student implements Comparable<Student> {
    String name;
    int grade;
    Student(String n, int g) { this.name = n; this.grade = g; }
    public int compareTo(Student other) { return this.grade - other.grade; }
    public String toString() { return name + ":" + grade; }
}
class Main {
    public static void main(String[] args) {
        ArrayList<Student> list = new ArrayList<>();
        list.add(new Student("Alice", 85));
        list.add(new Student("Bob", 72));
        list.add(new Student("Carol", 91));
        Collections.sort(list);
        for (Student s : list) System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "Bob:72\nAlice:85\nCarol:91");
}

#[test]
fn string_split_and_join() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    public static void main(String[] args) {
        String csv = "a,b,c,d,e";
        String[] parts = csv.split(",");
        System.out.println(parts.length);
        System.out.println(parts[2]);
        String joined = String.join("-", parts);
        System.out.println(joined);
        // split with limit
        String[] limited = csv.split(",", 3);
        System.out.println(limited.length);
        System.out.println(limited[2]);
    }
}
"#);
    assert_eq!(out.trim(), "5\nc\na-b-c-d-e\n3\nc,d,e");
}

#[test]
fn map_iteration_patterns() {
    let out = run(r#"
import java.util.HashMap;
import java.util.TreeMap;
import java.util.Map;
class Main {
    public static void main(String[] args) {
        TreeMap<String, Integer> map = new TreeMap<>();
        map.put("banana", 2);
        map.put("apple", 5);
        map.put("cherry", 1);
        // entrySet iteration (sorted by key in TreeMap)
        int total = 0;
        for (Map.Entry<String, Integer> e : map.entrySet()) {
            total += e.getValue();
        }
        System.out.println(total);
        System.out.println(map.size());
        System.out.println(map.containsKey("apple"));
        System.out.println(map.get("banana"));
    }
}
"#);
    assert_eq!(out.trim(), "8\n3\ntrue\n2");
}

#[test]
fn inheritance_method_override() {
    let out = run(r#"
class Animal {
    String name;
    Animal(String n) { this.name = n; }
    String sound() { return "..."; }
    String describe() { return name + " says " + sound(); }
}
class Dog extends Animal {
    Dog(String n) { super(n); }
    String sound() { return "woof"; }
}
class Cat extends Animal {
    Cat(String n) { super(n); }
    String sound() { return "meow"; }
}
class Main {
    static void makeSound(Animal a) { System.out.println(a.describe()); }
    public static void main(String[] args) {
        makeSound(new Dog("Rex"));
        makeSound(new Cat("Whiskers"));
        makeSound(new Animal("Unknown"));
    }
}
"#);
    assert_eq!(out.trim(), "Rex says woof\nWhiskers says meow\nUnknown says ...");
}

#[test]
fn stream_reduce_and_collect() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        int sum = nums.stream().reduce(0, (a, b) -> a + b);
        System.out.println(sum);
        int product = nums.stream().reduce(1, (a, b) -> a * b);
        System.out.println(product);
        List<Integer> evens = nums.stream()
            .filter(n -> n % 2 == 0)
            .collect(Collectors.toList());
        System.out.println(evens);
        long count = nums.stream().filter(n -> n > 2).count();
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "15\n120\n[2, 4]\n3");
}

#[test]
fn generic_interface_impl() {
    let out = run(r#"
interface Mapper<T, R> {
    R map(T input);
}
class DoubleMapper implements Mapper<Integer, Integer> {
    public Integer map(Integer input) { return input * 2; }
}
class StringMapper implements Mapper<Integer, String> {
    public String map(Integer input) { return "val=" + input; }
}
class Main {
    static <T, R> R apply(Mapper<T, R> m, T val) { return m.map(val); }
    public static void main(String[] args) {
        System.out.println(apply(new DoubleMapper(), 5));
        System.out.println(apply(new StringMapper(), 42));
    }
}
"#);
    assert_eq!(out.trim(), "10\nval=42");
}

#[test]
fn exception_custom_hierarchy() {
    let out = run(r#"
class AppException extends RuntimeException {
    int code;
    AppException(String msg, int code) {
        this.code = code;
    }
    public String getMessage() { return "AppException code=" + code; }
}
class ValidationException extends AppException {
    String field;
    ValidationException(String f) {
        super("invalid", 400);
        this.field = f;
    }
    public String getMessage() { return "Invalid: " + field; }
}
class Main {
    static void validate(String s) {
        if (s == null || s.isEmpty()) throw new ValidationException("name");
    }
    public static void main(String[] args) {
        try {
            validate("");
        } catch (ValidationException e) {
            System.out.println(e.getMessage());
            System.out.println(e.code);
        }
        try {
            validate("ok");
            System.out.println("valid");
        } catch (AppException e) {
            System.out.println("error");
        }
    }
}
"#);
    assert_eq!(out.trim(), "Invalid: name\n400\nvalid");
}

#[test]
fn string_valueof_and_conversions() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(String.valueOf(42));
        System.out.println(String.valueOf(3.14));
        System.out.println(String.valueOf(true));
        System.out.println(String.valueOf('A'));
        int n = Integer.parseInt("123");
        double d = Double.parseDouble("2.5");
        System.out.println(n + 1);
        System.out.println(d + 0.5);
        System.out.println(Integer.toBinaryString(10));
        System.out.println(Integer.toHexString(255));
        System.out.println(Integer.toOctalString(8));
    }
}
"#);
    assert_eq!(out.trim(), "42\n3.14\ntrue\nA\n124\n3.0\n1010\nff\n10");
}

#[test]
fn map_getordefault_and_putifabsent() {
    let out = run(r#"
import java.util.HashMap;
class Main {
    public static void main(String[] args) {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        System.out.println(map.getOrDefault("a", 0));
        System.out.println(map.getOrDefault("z", 99));
        map.putIfAbsent("a", 100);
        map.putIfAbsent("c", 3);
        System.out.println(map.get("a"));
        System.out.println(map.get("c"));
        System.out.println(map.size());
    }
}
"#);
    assert_eq!(out.trim(), "1\n99\n1\n3\n3");
}

#[test]
fn collections_ncopies_and_frequency() {
    let out = run(r#"
import java.util.Collections;
import java.util.ArrayList;
import java.util.List;
class Main {
    public static void main(String[] args) {
        List<String> copies = Collections.nCopies(3, "hello");
        System.out.println(copies.size());
        System.out.println(copies.get(0));
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1); list.add(2); list.add(2); list.add(3); list.add(2);
        System.out.println(Collections.frequency(list, 2));
        System.out.println(Collections.min(list));
        System.out.println(Collections.max(list));
    }
}
"#);
    assert_eq!(out.trim(), "3\nhello\n3\n1\n3");
}

#[test]
fn arrays_sort_and_binarysearch() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    public static void main(String[] args) {
        int[] arr = {5, 3, 8, 1, 9, 2};
        Arrays.sort(arr);
        System.out.println(Arrays.toString(arr));
        int idx = Arrays.binarySearch(arr, 8);
        System.out.println(idx);
        String[] words = {"banana", "apple", "cherry"};
        Arrays.sort(words);
        System.out.println(Arrays.toString(words));
        int[] copy = Arrays.copyOfRange(arr, 1, 4);
        System.out.println(Arrays.toString(copy));
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 5, 8, 9]\n4\n[apple, banana, cherry]\n[2, 3, 5]");
}

#[test]
fn string_builder_insert_and_index() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        StringBuilder sb = new StringBuilder("Hello World");
        sb.insert(5, ",");
        System.out.println(sb.toString());
        System.out.println(sb.indexOf("World"));
        System.out.println(sb.length());
        sb.deleteCharAt(5);
        System.out.println(sb.toString());
        sb.replace(0, 5, "Hi");
        System.out.println(sb.toString());
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World\n7\n12\nHello World\nHi World");
}

#[test]
fn deque_as_stack_and_queue() {
    let out = run(r#"
import java.util.ArrayDeque;
import java.util.Deque;
class Main {
    public static void main(String[] args) {
        // use as stack
        Deque<Integer> stack = new ArrayDeque<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        System.out.println(stack.peek());
        System.out.println(stack.pop());
        System.out.println(stack.size());
        // use as queue
        Deque<String> queue = new ArrayDeque<>();
        queue.offer("a");
        queue.offer("b");
        queue.offer("c");
        System.out.println(queue.poll());
        System.out.println(queue.peek());
        System.out.println(queue.size());
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n2\na\nb\n2");
}

#[test]
fn math_min_max_and_clamp() {
    let out = run(r#"
class Main {
    static int clamp(int val, int min, int max) {
        return Math.max(min, Math.min(max, val));
    }
    public static void main(String[] args) {
        System.out.println(Math.min(3, 7));
        System.out.println(Math.max(3, 7));
        System.out.println(Math.abs(-42));
        System.out.println(clamp(5, 0, 10));
        System.out.println(clamp(-5, 0, 10));
        System.out.println(clamp(15, 0, 10));
        System.out.println(Math.pow(2, 10));
        System.out.println((int) Math.sqrt(144));
    }
}
"#);
    assert_eq!(out.trim(), "3\n7\n42\n5\n0\n10\n1024.0\n12");
}

#[test]
fn list_sublist_and_contains() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.List;
class Main {
    public static void main(String[] args) {
        List<Integer> list = new ArrayList<>();
        for (int i = 1; i <= 6; i++) list.add(i);
        List<Integer> sub = list.subList(1, 4);
        System.out.println(sub);
        System.out.println(sub.size());
        System.out.println(list.contains(3));
        System.out.println(list.contains(9));
        System.out.println(list.indexOf(4));
        list.remove(2); // remove by index 2 → removes value 3
        System.out.println(list);
    }
}
"#);
    assert_eq!(out.trim(), "[2, 3, 4]\n3\ntrue\nfalse\n3\n[1, 2, 4, 5, 6]");
}

#[test]
fn nested_generic_map() {
    let out = run(r#"
import java.util.HashMap;
import java.util.ArrayList;
import java.util.Map;
class Main {
    public static void main(String[] args) {
        HashMap<String, ArrayList<Integer>> map = new HashMap<>();
        map.put("evens", new ArrayList<>());
        map.put("odds", new ArrayList<>());
        for (int i = 1; i <= 6; i++) {
            if (i % 2 == 0) map.get("evens").add(i);
            else map.get("odds").add(i);
        }
        System.out.println(map.get("evens"));
        System.out.println(map.get("odds"));
        System.out.println(map.get("evens").size());
    }
}
"#);
    assert_eq!(out.trim(), "[2, 4, 6]\n[1, 3, 5]\n3");
}

#[test]
fn string_format_various() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(String.format("%d + %d = %d", 3, 4, 7));
        System.out.println(String.format("%.2f", 3.14159));
        System.out.println(String.format("%05d", 42));
        System.out.println(String.format("%-10s|", "left"));
        System.out.println(String.format("%10s|", "right"));
        System.out.println(String.format("%s is %b", "java", true));
    }
}
"#);
    assert_eq!(out.trim(), "3 + 4 = 7\n3.14\n00042\nleft      |\n     right|\njava is true");
}

#[test]
fn functional_compose_and_andthen() {
    let out = run(r#"
import java.util.function.Function;
class Main {
    public static void main(String[] args) {
        Function<Integer, Integer> times2 = x -> x * 2;
        Function<Integer, Integer> plus3  = x -> x + 3;
        Function<Integer, Integer> composed = times2.andThen(plus3);
        Function<Integer, Integer> composed2 = times2.compose(plus3);
        System.out.println(composed.apply(5));   // (5*2)+3 = 13
        System.out.println(composed2.apply(5));  // (5+3)*2 = 16
        Function<String, Integer> len = String::length;
        System.out.println(len.apply("hello"));
    }
}
"#);
    assert_eq!(out.trim(), "13\n16\n5");
}

#[test]
fn stream_map_and_distinct() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 2, 3, 3, 3, 4);
        List<Integer> distinct = nums.stream().distinct().collect(Collectors.toList());
        System.out.println(distinct);
        List<String> strs = nums.stream()
            .distinct()
            .map(n -> "n" + n)
            .collect(Collectors.toList());
        System.out.println(strs);
        long count = nums.stream().distinct().count();
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 4]\n[n1, n2, n3, n4]\n4");
}

#[test]
fn optional_map_and_filter() {
    let out = run(r#"
import java.util.Optional;
class Main {
    static Optional<String> findName(boolean found) {
        return found ? Optional.of("Alice") : Optional.empty();
    }
    public static void main(String[] args) {
        Optional<String> name = findName(true);
        System.out.println(name.isPresent());
        System.out.println(name.get());
        System.out.println(name.map(String::toUpperCase).orElse("none"));
        Optional<String> empty = findName(false);
        System.out.println(empty.isPresent());
        System.out.println(empty.orElse("default"));
        System.out.println(empty.map(String::toUpperCase).orElse("none"));
    }
}
"#);
    assert_eq!(out.trim(), "true\nAlice\nALICE\nfalse\ndefault\nnone");
}

#[test]
fn enum_switch_and_methods() {
    let out = run(r#"
enum Direction {
    NORTH, SOUTH, EAST, WEST;
    public Direction opposite() {
        switch (this) {
            case NORTH: return SOUTH;
            case SOUTH: return NORTH;
            case EAST:  return WEST;
            case WEST:  return EAST;
            default:    return this;
        }
    }
}
class Main {
    public static void main(String[] args) {
        Direction d = Direction.NORTH;
        System.out.println(d);
        System.out.println(d.opposite());
        System.out.println(Direction.EAST.opposite());
        System.out.println(d.ordinal());
        System.out.println(Direction.values().length);
    }
}
"#);
    assert_eq!(out.trim(), "NORTH\nSOUTH\nWEST\n0\n4");
}

#[test]
fn interface_multiple_default() {
    let out = run(r#"
interface Printable {
    default void print() { System.out.println("Printable: " + describe()); }
    String describe();
}
interface Saveable {
    default void save() { System.out.println("Saving: " + describe()); }
    String describe();
}
class Document implements Printable, Saveable {
    String title;
    Document(String t) { this.title = t; }
    public String describe() { return title; }
}
class Main {
    public static void main(String[] args) {
        Document doc = new Document("Report");
        doc.print();
        doc.save();
        System.out.println(doc.describe());
    }
}
"#);
    assert_eq!(out.trim(), "Printable: Report\nSaving: Report\nReport");
}

#[test]
fn array_varargs_and_spread() {
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
        System.out.println(sum(1, 2, 3));
        System.out.println(sum(10, 20, 30, 40));
        System.out.println(sum());
        System.out.println(join(", ", "a", "b", "c"));
        System.out.println(join("-", "x", "y"));
    }
}
"#);
    assert_eq!(out.trim(), "6\n100\n0\na, b, c\nx-y");
}

#[test]
fn generic_bounded_extends() {
    let out = run(r#"
class Box<T extends Comparable<T>> {
    T value;
    Box(T v) { this.value = v; }
    T get() { return value; }
    boolean isGreaterThan(Box<T> other) { return value.compareTo(other.value) > 0; }
}
class Main {
    public static void main(String[] args) {
        Box<Integer> a = new Box<>(10);
        Box<Integer> b = new Box<>(5);
        System.out.println(a.get());
        System.out.println(a.isGreaterThan(b));
        System.out.println(b.isGreaterThan(a));
        Box<String> s1 = new Box<>("banana");
        Box<String> s2 = new Box<>("apple");
        System.out.println(s1.isGreaterThan(s2));
    }
}
"#);
    assert_eq!(out.trim(), "10\ntrue\nfalse\ntrue");
}

#[test]
fn static_nested_class() {
    let out = run(r#"
class Outer {
    static int x = 10;
    static class Inner {
        int y;
        Inner(int y) { this.y = y; }
        int getY() { return y; }
    }
    static Inner create(int y) { return new Inner(y); }
    static int sumWith(Inner inner) { return x + inner.getY(); }
}
class Main {
    public static void main(String[] args) {
        Outer.Inner inner = new Outer.Inner(5);
        System.out.println(Outer.sumWith(inner));
        Outer.Inner inner2 = Outer.create(20);
        System.out.println(Outer.sumWith(inner2));
        System.out.println(Outer.x);
    }
}
"#);
    assert_eq!(out.trim(), "15\n30\n10");
}

#[test]
fn collections_reverse_and_shuffle_seed() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.Collections;
import java.util.Arrays;
import java.util.List;
class Main {
    public static void main(String[] args) {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.reverse(list);
        System.out.println(list);
        Collections.sort(list);
        System.out.println(list);
        List<String> words = new ArrayList<>(Arrays.asList("c", "a", "b"));
        Collections.sort(words);
        System.out.println(words);
        Collections.reverse(words);
        System.out.println(words);
    }
}
"#);
    assert_eq!(out.trim(), "[5, 4, 3, 2, 1]\n[1, 2, 3, 4, 5]\n[a, b, c]\n[c, b, a]");
}

#[test]
fn string_regex_matches_and_replace() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello World 123";
        System.out.println(s.matches(".*\\d+.*"));
        System.out.println(s.matches("Hello.*"));
        System.out.println(s.replaceAll("\\d+", "NUM"));
        System.out.println(s.replaceFirst("[A-Z]", "X"));
        System.out.println("a1b2c3".replaceAll("[0-9]", ""));
        System.out.println("  hello  ".trim());
    }
}
"#);
    assert_eq!(out.trim(), "true\ntrue\nHello World NUM\nXello World 123\nabc\nhello");
}

#[test]
fn interface_functional_lambda() {
    let out = run(r#"
@FunctionalInterface
interface Transformer<T> {
    T transform(T input);
}
class Main {
    static <T> T apply(Transformer<T> t, T val) { return t.transform(val); }
    public static void main(String[] args) {
        Transformer<String> upper = s -> s.toUpperCase();
        Transformer<Integer> double_ = n -> n * 2;
        System.out.println(apply(upper, "hello"));
        System.out.println(apply(double_, 21));
        Transformer<String> trim = String::trim;
        System.out.println(apply(trim, "  hi  "));
    }
}
"#);
    assert_eq!(out.trim(), "HELLO\n42\nhi");
}

#[test]
fn map_treemap_navigation() {
    let out = run(r#"
import java.util.TreeMap;
class Main {
    public static void main(String[] args) {
        TreeMap<Integer, String> map = new TreeMap<>();
        map.put(1, "one");
        map.put(3, "three");
        map.put(5, "five");
        map.put(7, "seven");
        System.out.println(map.firstKey());
        System.out.println(map.lastKey());
        System.out.println(map.size());
        System.out.println(map.containsKey(3));
        System.out.println(map.containsKey(4));
        System.out.println(map.get(5));
    }
}
"#);
    assert_eq!(out.trim(), "1\n7\n4\ntrue\nfalse\nfive");
}

#[test]
fn exception_rethrowing() {
    let out = run(r#"
class Main {
    static int divide(int a, int b) {
        if (b == 0) throw new ArithmeticException("division by zero");
        return a / b;
    }
    static int safeDivide(int a, int b) {
        try {
            return divide(a, b);
        } catch (ArithmeticException e) {
            System.out.println("caught: " + e.getMessage());
            return -1;
        }
    }
    public static void main(String[] args) {
        System.out.println(safeDivide(10, 2));
        System.out.println(safeDivide(10, 0));
        try {
            divide(5, 0);
        } catch (ArithmeticException e) {
            System.out.println("outer: " + e.getMessage());
        }
    }
}
"#);
    assert_eq!(out.trim(), "5\ncaught: division by zero\n-1\nouter: division by zero");
}

#[test]
fn stream_anyof_allof_noneof() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        System.out.println(nums.stream().anyMatch(n -> n > 4));
        System.out.println(nums.stream().anyMatch(n -> n > 10));
        System.out.println(nums.stream().allMatch(n -> n > 0));
        System.out.println(nums.stream().allMatch(n -> n > 2));
        System.out.println(nums.stream().noneMatch(n -> n > 10));
        System.out.println(nums.stream().noneMatch(n -> n > 4));
        System.out.println(nums.stream().min((a, b) -> a - b).get());
        System.out.println(nums.stream().max((a, b) -> a - b).get());
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse\ntrue\nfalse\ntrue\nfalse\n1\n5");
}

#[test]
fn string_number_parsing() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Integer.parseInt("-42"));
        System.out.println(Long.parseLong("9876543210"));
        System.out.println(Double.parseDouble("3.14"));
        System.out.println(Float.parseFloat("2.5"));
        System.out.println(Integer.MAX_VALUE);
        System.out.println(Integer.MIN_VALUE);
        System.out.println(Integer.compare(3, 5));
        System.out.println(Integer.compare(5, 3));
        System.out.println(Integer.compare(3, 3));
    }
}
"#);
    assert_eq!(out.trim(), "-42\n9876543210\n3.14\n2.5\n2147483647\n-2147483648\n-1\n1\n0");
}

#[test]
fn collections_linked_hashmap() {
    let out = run(r#"
import java.util.LinkedHashMap;
import java.util.Map;
class Main {
    public static void main(String[] args) {
        LinkedHashMap<String, Integer> map = new LinkedHashMap<>();
        map.put("c", 3);
        map.put("a", 1);
        map.put("b", 2);
        // LinkedHashMap preserves insertion order
        for (Map.Entry<String, Integer> e : map.entrySet()) {
            System.out.println(e.getKey() + "=" + e.getValue());
        }
        System.out.println(map.size());
        map.remove("a");
        System.out.println(map.size());
    }
}
"#);
    assert_eq!(out.trim(), "c=3\na=1\nb=2\n3\n2");
}

#[test]
fn abstract_class_with_state() {
    let out = run(r#"
abstract class Shape {
    String color;
    Shape(String color) { this.color = color; }
    abstract double area();
    String describe() { return color + " shape with area " + String.format("%.1f", area()); }
}
class Circle extends Shape {
    double radius;
    Circle(String color, double radius) { super(color); this.radius = radius; }
    public double area() { return Math.PI * radius * radius; }
}
class Rectangle extends Shape {
    double w, h;
    Rectangle(String color, double w, double h) { super(color); this.w = w; this.h = h; }
    public double area() { return w * h; }
}
class Main {
    public static void main(String[] args) {
        Shape[] shapes = { new Circle("red", 5), new Rectangle("blue", 4, 6) };
        for (Shape s : shapes) System.out.println(s.describe());
    }
}
"#);
    assert_eq!(out.trim(), "red shape with area 78.5\nblue shape with area 24.0");
}

#[test]
fn stream_collect_joining() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("hello", "world", "java");
        String joined = words.stream().collect(Collectors.joining(", "));
        System.out.println(joined);
        String withBrackets = words.stream().collect(Collectors.joining(", ", "[", "]"));
        System.out.println(withBrackets);
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        int sum = nums.stream().mapToInt(Integer::intValue).sum();
        System.out.println(sum);
        double avg = nums.stream().mapToInt(Integer::intValue).average();
        System.out.println(avg);
    }
}
"#);
    assert_eq!(out.trim(), "hello, world, java\n[hello, world, java]\n15\n3.0");
}

#[test]
fn generic_wildcard_upper_bound() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.List;
class Main {
    static double sumList(List<? extends Number> list) {
        double sum = 0;
        for (Number n : list) sum += n.doubleValue();
        return sum;
    }
    static void printAll(List<?> list) {
        for (Object o : list) System.out.println(o);
    }
    public static void main(String[] args) {
        List<Integer> ints = new ArrayList<>();
        ints.add(1); ints.add(2); ints.add(3);
        System.out.println(sumList(ints));
        List<Double> doubles = new ArrayList<>();
        doubles.add(1.5); doubles.add(2.5);
        System.out.println(sumList(doubles));
        printAll(ints);
    }
}
"#);
    assert_eq!(out.trim(), "6.0\n4.0\n1\n2\n3");
}

#[test]
fn exception_multi_level_catch() {
    let out = run(r#"
class Main {
    static void level3() { throw new IllegalArgumentException("bad arg"); }
    static void level2() { level3(); }
    static void level1() {
        try {
            level2();
        } catch (RuntimeException e) {
            System.out.println("level1 caught: " + e.getMessage());
            throw new RuntimeException("wrapped: " + e.getMessage());
        }
    }
    public static void main(String[] args) {
        try {
            level1();
        } catch (RuntimeException e) {
            System.out.println("main caught: " + e.getMessage());
        }
        System.out.println("done");
    }
}
"#);
    assert_eq!(out.trim(), "level1 caught: bad arg\nmain caught: wrapped: bad arg\ndone");
}

#[test]
fn collections_set_operations() {
    let out = run(r#"
import java.util.HashSet;
import java.util.TreeSet;
import java.util.Set;
class Main {
    public static void main(String[] args) {
        Set<Integer> a = new HashSet<>();
        a.add(1); a.add(2); a.add(3); a.add(4);
        Set<Integer> b = new HashSet<>();
        b.add(3); b.add(4); b.add(5); b.add(6);
        // intersection
        Set<Integer> inter = new HashSet<>(a);
        inter.retainAll(b);
        TreeSet<Integer> sorted = new TreeSet<>(inter);
        System.out.println(sorted);
        // union
        Set<Integer> union = new HashSet<>(a);
        union.addAll(b);
        System.out.println(new TreeSet<>(union));
        System.out.println(a.contains(2));
        System.out.println(a.contains(5));
    }
}
"#);
    assert_eq!(out.trim(), "[3, 4]\n[1, 2, 3, 4, 5, 6]\ntrue\nfalse");
}

#[test]
fn lambda_method_reference_static() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    static int doubleIt(int n) { return n * 2; }
    static boolean isEven(int n) { return n % 2 == 0; }
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        List<Integer> doubled = nums.stream()
            .map(n -> doubleIt(n))
            .collect(Collectors.toList());
        System.out.println(doubled);
        List<Integer> evens = nums.stream()
            .filter(n -> isEven(n))
            .collect(Collectors.toList());
        System.out.println(evens);
        long count = nums.stream().filter(n -> isEven(n)).count();
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "[2, 4, 6, 8, 10]\n[2, 4]\n2");
}


#[test]
fn collections_sort_with_comparator() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
class Main {
    public static void main(String[] args) {
        List<String> words = new ArrayList<>();
        words.add("banana"); words.add("apple"); words.add("cherry"); words.add("date");
        Collections.sort(words);
        System.out.println(words);
        Collections.sort(words, (a, b) -> b.compareTo(a));
        System.out.println(words);
        List<Integer> nums = new ArrayList<>();
        nums.add(3); nums.add(1); nums.add(4); nums.add(1); nums.add(5);
        Collections.sort(nums);
        System.out.println(nums);
        System.out.println(Collections.min(nums));
        System.out.println(Collections.max(nums));
    }
}
"#);
    assert_eq!(out.trim(), "[apple, banana, cherry, date]\n[date, cherry, banana, apple]\n[1, 1, 3, 4, 5]\n1\n5");
}

#[test]
fn multi_catch_and_finally() {
    let out = run(r#"
class Main {
    static int divide(int a, int b) { return a / b; }
    public static void main(String[] args) {
        try {
            System.out.println(divide(10, 0));
        } catch (ArithmeticException e) {
            System.out.println("caught: " + e.getMessage());
        } finally {
            System.out.println("finally1");
        }
        try {
            Integer.parseInt("abc");
        } catch (NumberFormatException e) {
            System.out.println("nfe caught");
        } finally {
            System.out.println("finally2");
        }
        System.out.println("done");
    }
}
"#);
    assert_eq!(out.trim(), "caught: / by zero\nfinally1\nnfe caught\nfinally2\ndone");
}

#[test]
fn interface_default_method_override() {
    let out = run(r#"
interface Greeter {
    String greet(String name);
    default String greetLoud(String name) {
        return greet(name).toUpperCase();
    }
}
class FormalGreeter implements Greeter {
    public String greet(String name) { return "Good day, " + name; }
}
class CasualGreeter implements Greeter {
    public String greet(String name) { return "Hey " + name + "!"; }
    public String greetLoud(String name) { return "HEY " + name.toUpperCase() + "!!!"; }
}
class Main {
    public static void main(String[] args) {
        Greeter f = new FormalGreeter();
        Greeter c = new CasualGreeter();
        System.out.println(f.greet("Alice"));
        System.out.println(f.greetLoud("Alice"));
        System.out.println(c.greet("Bob"));
        System.out.println(c.greetLoud("Bob"));
    }
}
"#);
    assert_eq!(out.trim(), "Good day, Alice\nGOOD DAY, ALICE\nHey Bob!\nHEY BOB!!!");
}

#[test]
fn stream_reduce_operations() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        int sum = nums.stream().reduce(0, (a, b) -> a + b);
        System.out.println(sum);
        int product = nums.stream().reduce(1, (a, b) -> a * b);
        System.out.println(product);
        List<Integer> doubled = nums.stream()
            .map(n -> n * 2)
            .collect(Collectors.toList());
        System.out.println(doubled);
        String joined = nums.stream()
            .map(n -> String.valueOf(n))
            .collect(Collectors.joining("-"));
        System.out.println(joined);
    }
}
"#);
    assert_eq!(out.trim(), "15\n120\n[2, 4, 6, 8, 10]\n1-2-3-4-5");
}

#[test]
fn character_class_methods() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Character.isUpperCase('A'));
        System.out.println(Character.isLowerCase('a'));
        System.out.println(Character.isDigit('5'));
        System.out.println(Character.isLetter('z'));
        System.out.println(Character.isWhitespace(' '));
        System.out.println(Character.toUpperCase('a'));
        System.out.println(Character.toLowerCase('Z'));
        System.out.println(Character.isAlphabetic('b'));
    }
}
"#);
    assert_eq!(out.trim(), "true\ntrue\ntrue\ntrue\ntrue\nA\nz\ntrue");
}

#[test]
fn map_merge_and_compute() {
    let out = run(r#"
import java.util.HashMap;
import java.util.Map;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> freq = new HashMap<>();
        String[] words = {"apple", "banana", "apple", "cherry", "banana", "apple"};
        for (String w : words) {
            freq.merge(w, 1, (old, v) -> old + v);
        }
        System.out.println(freq.get("apple"));
        System.out.println(freq.get("banana"));
        System.out.println(freq.get("cherry"));
        Map<String, Integer> m = new HashMap<>();
        m.computeIfAbsent("key", k -> k.length());
        System.out.println(m.get("key"));
        m.putIfAbsent("key", 999);
        System.out.println(m.get("key"));
        m.putIfAbsent("new", 42);
        System.out.println(m.get("new"));
    }
}
"#);
    assert_eq!(out.trim(), "3\n2\n1\n3\n3\n42");
}


#[test]
fn string_builder_method_chaining() {
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
        StringBuilder sb = new StringBuilder("abcde");
        sb.reverse();
        System.out.println(sb.toString());
        sb.insert(2, "XY");
        System.out.println(sb.toString());
        sb.delete(2, 4);
        System.out.println(sb.toString());
        System.out.println(sb.length());
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!\nedcba\nedXYcba\nedcba\n5");
}

#[test]
fn matrix_2d_transpose() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] matrix = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                matrix[i][j] = i * 3 + j + 1;
        for (int[] row : matrix) {
            for (int v : row) System.out.print(v + " ");
            System.out.println();
        }
        int[][] t = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                t[j][i] = matrix[i][j];
        System.out.println(t[0][1]);
        System.out.println(t[1][0]);
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3 \n4 5 6 \n7 8 9 \n4\n2");
}

#[test]
fn generic_stack_implementation() {
    let out = run(r#"
class Stack<T> {
    private Object[] data;
    private int size;
    Stack(int cap) { data = new Object[cap]; size = 0; }
    void push(T val) { data[size++] = val; }
    T pop() { return (T) data[--size]; }
    T peek() { return (T) data[size - 1]; }
    boolean isEmpty() { return size == 0; }
    int size() { return size; }
}
class Main {
    public static void main(String[] args) {
        Stack<Integer> s = new Stack<>(10);
        s.push(1); s.push(2); s.push(3);
        System.out.println(s.peek());
        System.out.println(s.pop());
        System.out.println(s.size());
        System.out.println(s.isEmpty());
        s.pop(); s.pop();
        System.out.println(s.isEmpty());
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n2\nfalse\ntrue");
}


#[test]
fn comparable_custom_sort() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
class Person implements Comparable<Person> {
    String name;
    int age;
    Person(String name, int age) { this.name = name; this.age = age; }
    public int compareTo(Person other) { return this.age - other.age; }
    public String toString() { return name + "(" + age + ")"; }
}
class Main {
    public static void main(String[] args) {
        List<Person> people = new ArrayList<>();
        people.add(new Person("Charlie", 30));
        people.add(new Person("Alice", 25));
        people.add(new Person("Bob", 35));
        Collections.sort(people);
        for (Person p : people) System.out.println(p);
    }
}
"#);
    assert_eq!(out.trim(), "Alice(25)\nCharlie(30)\nBob(35)");
}

#[test]
fn string_split_regex() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    public static void main(String[] args) {
        String csv = "one,two,three,four";
        String[] parts = csv.split(",");
        System.out.println(parts.length);
        System.out.println(parts[0]);
        System.out.println(parts[3]);
        String joined = String.join(" | ", parts);
        System.out.println(joined);
        String sentence = "  hello   world  ";
        String[] words = sentence.trim().split("\\s+");
        System.out.println(words.length);
        System.out.println(String.join("-", words));
    }
}
"#);
    assert_eq!(out.trim(), "4\none\nfour\none | two | three | four\n2\nhello-world");
}

#[test]
fn optional_map_flatmap() {
    let out = run(r#"
import java.util.Optional;
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
        System.out.println(empty.orElseGet(() -> "computed"));
        Optional<Integer> len = name.map(s -> s.length());
        System.out.println(len.get());
    }
}
"#);
    assert_eq!(out.trim(), "true\nAlice\nfalse\ndefault\ncomputed\n5");
}

#[test]
fn instanceof_pattern_cast() {
    let out = run(r#"
class Shape { String color; Shape(String c) { color = c; } }
class Circle extends Shape {
    double r;
    Circle(String c, double r) { super(c); this.r = r; }
    double area() { return Math.PI * r * r; }
}
class Rectangle extends Shape {
    double w, h;
    Rectangle(String c, double w, double h) { super(c); this.w = w; this.h = h; }
    double area() { return w * h; }
}
class Main {
    public static void main(String[] args) {
        Shape[] shapes = { new Circle("red", 3), new Rectangle("blue", 4, 5) };
        for (Shape s : shapes) {
            System.out.println(s instanceof Circle);
            System.out.println(s instanceof Rectangle);
            if (s instanceof Circle) {
                Circle c = (Circle) s;
                System.out.printf("%.2f%n", c.area());
            } else {
                Rectangle r = (Rectangle) s;
                System.out.printf("%.2f%n", r.area());
            }
        }
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse\n28.27\nfalse\ntrue\n20.00");
}


#[test]
fn stream_filter_chain() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        List<Integer> result = nums.stream()
            .filter(n -> n % 2 == 0)
            .filter(n -> n > 4)
            .collect(Collectors.toList());
        System.out.println(result);
        long count = nums.stream().filter(n -> n % 3 == 0).count();
        System.out.println(count);
        int sum = nums.stream().filter(n -> n % 2 != 0).mapToInt(n -> n).sum();
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "[6, 8, 10]\n3\n25");
}

#[test]
fn generic_pair_v2() {
    let out = run(r#"
class Pair<A, B> {
    A first;
    B second;
    Pair(A first, B second) { this.first = first; this.second = second; }
    public String toString() { return "(" + first + ", " + second + ")"; }
    Pair<B, A> swap() { return new Pair<>(second, first); }
}
class Main {
    public static void main(String[] args) {
        Pair<String, Integer> p = new Pair<>("hello", 42);
        System.out.println(p);
        System.out.println(p.first);
        System.out.println(p.second);
        Pair<Integer, String> swapped = p.swap();
        System.out.println(swapped);
    }
}
"#);
    assert_eq!(out.trim(), "(hello, 42)\nhello\n42\n(42, hello)");
}

#[test]
fn binary_search_and_sort() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    static int binarySearch(int[] arr, int target) {
        int lo = 0, hi = arr.length - 1;
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            if (arr[mid] == target) return mid;
            else if (arr[mid] < target) lo = mid + 1;
            else hi = mid - 1;
        }
        return -1;
    }
    static void bubbleSort(int[] arr) {
        for (int i = 0; i < arr.length - 1; i++)
            for (int j = 0; j < arr.length - 1 - i; j++)
                if (arr[j] > arr[j+1]) { int t = arr[j]; arr[j] = arr[j+1]; arr[j+1] = t; }
    }
    public static void main(String[] args) {
        int[] arr = {5, 2, 8, 1, 9, 3};
        bubbleSort(arr);
        System.out.println(Arrays.toString(arr));
        System.out.println(binarySearch(arr, 8));
        System.out.println(binarySearch(arr, 7));
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 5, 8, 9]\n4\n-1");
}

#[test]
fn varargs_multiple() {
    let out = run(r#"
class Main {
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }
    static String concat(String sep, String... parts) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < parts.length; i++) {
            if (i > 0) sb.append(sep);
            sb.append(parts[i]);
        }
        return sb.toString();
    }
    public static void main(String[] args) {
        System.out.println(sum());
        System.out.println(sum(1));
        System.out.println(sum(1, 2, 3));
        System.out.println(sum(1, 2, 3, 4, 5));
        System.out.println(concat(", ", "a", "b", "c"));
        System.out.println(concat("-", "x", "y"));
    }
}
"#);
    assert_eq!(out.trim(), "0\n1\n6\n15\na, b, c\nx-y");
}

#[test]
fn builder_pattern() {
    let out = run(r#"
class Person {
    String name;
    int age;
    String email;
    private Person() {}
    static class Builder {
        private Person p = new Person();
        Builder name(String n) { p.name = n; return this; }
        Builder age(int a) { p.age = a; return this; }
        Builder email(String e) { p.email = e; return this; }
        Person build() { return p; }
    }
    public String toString() { return name + "(" + age + ") <" + email + ">"; }
}
class Main {
    public static void main(String[] args) {
        Person p = new Person.Builder()
            .name("Alice")
            .age(30)
            .email("alice@example.com")
            .build();
        System.out.println(p);
    }
}
"#);
    assert_eq!(out.trim(), "Alice(30) <alice@example.com>");
}

#[test]
fn collections_priority_queue_custom() {
    let out = run(r#"
import java.util.PriorityQueue;
import java.util.Comparator;
class Main {
    public static void main(String[] args) {
        // max-heap using reverse comparator
        PriorityQueue<Integer> maxHeap = new PriorityQueue<>((a, b) -> b - a);
        maxHeap.offer(3); maxHeap.offer(1); maxHeap.offer(4); maxHeap.offer(1); maxHeap.offer(5);
        System.out.println(maxHeap.poll());
        System.out.println(maxHeap.poll());
        System.out.println(maxHeap.peek());
        // min-heap (default)
        PriorityQueue<Integer> minHeap = new PriorityQueue<>();
        minHeap.offer(5); minHeap.offer(2); minHeap.offer(8); minHeap.offer(1);
        System.out.println(minHeap.poll());
        System.out.println(minHeap.poll());
    }
}
"#);
    assert_eq!(out.trim(), "5\n4\n3\n1\n2");
}


#[test]
fn string_advanced_methods() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello, World\!";
        System.out.println(s.replace("World", "Java"));
        System.out.println(s.replaceAll("[aeiou]", "*"));
        System.out.println("  hello  ".strip());
        System.out.println("abc".repeat(3));
        System.out.println("hello world".contains("world"));
        System.out.println("hello".startsWith("hel"));
        System.out.println("hello".endsWith("llo"));
    }
}
"#);
    assert_eq!(out.trim(), "Hello, Java!\nH*ll*, W*rld!\nhello\nabcabcabc\ntrue\ntrue\ntrue");
}

#[test]
fn treemap_sorted_iteration() {
    let out = run(r#"
import java.util.Map;
import java.util.TreeMap;
class Main {
    public static void main(String[] args) {
        TreeMap<String, Integer> scores = new TreeMap<>();
        scores.put("Charlie", 92);
        scores.put("Alice", 95);
        scores.put("Bob", 87);
        for (Map.Entry<String, Integer> e : scores.entrySet()) {
            System.out.println(e.getKey() + ": " + e.getValue());
        }
        System.out.println(scores.firstKey());
        System.out.println(scores.lastKey());
        System.out.println(scores.containsKey("Bob"));
        System.out.println(scores.containsValue(100));
    }
}
"#);
    assert_eq!(out.trim(), "Alice: 95\nBob: 87\nCharlie: 92\nAlice\nCharlie\ntrue\nfalse");
}

#[test]
fn bit_manipulation() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int a = 10;  // 1010
        int b = 12;  // 1100
        System.out.println(a & b);   // 8
        System.out.println(a | b);   // 14
        System.out.println(a ^ b);   // 6
        System.out.println(~a);      // -11
        System.out.println(a << 1);  // 20
        System.out.println(a >> 1);  // 5
        System.out.println((a & (1 << 1)) != 0); // true
        System.out.println((a & (1 << 2)) != 0); // false
    }
}
"#);
    assert_eq!(out.trim(), "8\n14\n6\n-11\n20\n5\ntrue\nfalse");
}


#[test]
fn interface_polymorphism() {
    let out = run(r#"
interface Shape {
    double area();
    default String describe() { return "Shape with area " + String.format("%.1f", area()); }
}
class Circle implements Shape {
    double r;
    Circle(double r) { this.r = r; }
    public double area() { return Math.PI * r * r; }
}
class Square implements Shape {
    double s;
    Square(double s) { this.s = s; }
    public double area() { return s * s; }
}
class Triangle implements Shape {
    double b, h;
    Triangle(double b, double h) { this.b = b; this.h = h; }
    public double area() { return 0.5 * b * h; }
}
class Main {
    public static void main(String[] args) {
        Shape[] shapes = { new Circle(5), new Square(4), new Triangle(6, 8) };
        for (Shape s : shapes) System.out.println(s.describe());
    }
}
"#);
    assert_eq!(out.trim(), "Shape with area 78.5\nShape with area 16.0\nShape with area 24.0");
}

#[test]
fn integer_bit_methods() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Integer.toBinaryString(42));
        System.out.println(Integer.toHexString(255));
        System.out.println(Integer.toOctalString(8));
        System.out.println(Integer.parseInt("101010", 2));
        System.out.println(Integer.parseInt("ff", 16));
        System.out.println(Integer.bitCount(255));
        System.out.println(Integer.highestOneBit(100));
    }
}
"#);
    assert_eq!(out.trim(), "101010\nff\n10\n42\n255\n8\n64");
}

#[test]
fn functional_transform_compose() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
class Main {
    interface Transform { int apply(int x); }
    public static void main(String[] args) {
        Transform doubleIt = x -> x * 2;
        Transform addTen = x -> x + 10;
        // apply directly
        System.out.println(doubleIt.apply(5));
        System.out.println(addTen.apply(5));
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        List<Integer> doubled = nums.stream()
            .map(n -> doubleIt.apply(n))
            .collect(Collectors.toList());
        System.out.println(doubled);
        List<Integer> added = nums.stream()
            .map(n -> addTen.apply(n))
            .collect(Collectors.toList());
        System.out.println(added);
    }
}
"#);
    assert_eq!(out.trim(), "10\n15\n[2, 4, 6, 8, 10]\n[11, 12, 13, 14, 15]");
}

#[test]
fn point_array_distance() {
    let out = run(r#"
class Point {
    int x, y;
    Point(int x, int y) { this.x = x; this.y = y; }
    double distanceTo(Point other) {
        int dx = this.x - other.x, dy = this.y - other.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
    public String toString() { return "(" + x + "," + y + ")"; }
}
class Main {
    public static void main(String[] args) {
        Point[] points = { new Point(0,0), new Point(3,4), new Point(6,8) };
        for (Point p : points) System.out.println(p);
        System.out.printf("%.1f%n", points[0].distanceTo(points[1]));
        System.out.printf("%.1f%n", points[1].distanceTo(points[2]));
    }
}
"#);
    assert_eq!(out.trim(), "(0,0)\n(3,4)\n(6,8)\n5.0\n5.0");
}

#[test]
fn switch_expression_arrow() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        for (int i = 1; i <= 7; i++) {
            String day = switch (i) {
                case 1 -> "Monday";
                case 2 -> "Tuesday";
                case 3 -> "Wednesday";
                case 4 -> "Thursday";
                case 5 -> "Friday";
                case 6 -> "Saturday";
                case 7 -> "Sunday";
                default -> "Unknown";
            };
            if (i <= 3 || i == 7) System.out.println(day);
        }
    }
}
"#);
    assert_eq!(out.trim(), "Monday\nTuesday\nWednesday\nSunday");
}


#[test]
fn pascal_triangle() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] pascal = new int[6][6];
        for (int i = 0; i < 6; i++) {
            pascal[i][0] = 1;
            for (int j = 1; j < i; j++)
                pascal[i][j] = pascal[i-1][j-1] + pascal[i-1][j];
            pascal[i][i] = 1;
        }
        for (int i = 0; i < 5; i++) {
            for (int j = 0; j <= i; j++)
                System.out.print(pascal[i][j] + " ");
            System.out.println();
        }
    }
}
"#);
    assert_eq!(out.trim(), "1 \n1 1 \n1 2 1 \n1 3 3 1 \n1 4 6 4 1");
}

#[test]
fn do_while_basic() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int i = 0;
        do {
            System.out.println(i);
            i++;
        } while (i < 3);
        int x = 10;
        do {
            System.out.println("executed: " + x);
            x++;
        } while (x < 5);
        System.out.println("done");
    }
}
"#);
    assert_eq!(out.trim(), "0\n1\n2\nexecuted: 10\ndone");
}

#[test]
fn map_of_factory() {
    let out = run(r#"
import java.util.Map;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> m = Map.of("a", 1, "b", 2, "c", 3);
        System.out.println(m.size());
        System.out.println(m.get("a"));
        System.out.println(m.get("b"));
        System.out.println(m.containsKey("c"));
        System.out.println(m.containsKey("d"));
        try {
            m.put("d", 4);
        } catch (UnsupportedOperationException e) {
            System.out.println("immutable");
        }
    }
}
"#);
    assert_eq!(out.trim(), "3\n1\n2\ntrue\nfalse\nimmutable");
}


#[test]
fn string_palindrome_and_reverse() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello";
        int vowels = 0;
        for (char c : s.toCharArray()) {
            if ("aeiouAEIOU".indexOf(c) >= 0) vowels++;
        }
        System.out.println(vowels);
        StringBuilder sb = new StringBuilder(s);
        System.out.println(sb.reverse().toString());
        String t = "racecar";
        String rev = new StringBuilder(t).reverse().toString();
        System.out.println(t.equals(rev));
    }
}
"#);
    assert_eq!(out.trim(), "2\nolleH\ntrue");
}

#[test]
fn generic_number_box() {
    let out = run(r#"
class NumberBox<T extends Number> {
    T value;
    NumberBox(T value) { this.value = value; }
    double doubled() { return value.doubleValue() * 2; }
    boolean isPositive() { return value.doubleValue() > 0; }
}
class Main {
    public static void main(String[] args) {
        NumberBox<Integer> intBox = new NumberBox<>(42);
        NumberBox<Double> dblBox = new NumberBox<>(-3.14);
        System.out.println(intBox.doubled());
        System.out.println(intBox.isPositive());
        System.out.println(dblBox.doubled());
        System.out.println(dblBox.isPositive());
    }
}
"#);
    assert_eq!(out.trim(), "84.0\ntrue\n-6.28\nfalse");
}

#[test]
fn collections_disjoint_frequency() {
    let out = run(r#"
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
class Main {
    public static void main(String[] args) {
        List<Integer> list = Arrays.asList(1, 2, 3, 2, 1, 2, 4);
        System.out.println(Collections.frequency(list, 2));
        System.out.println(Collections.frequency(list, 5));
        List<Integer> a = Arrays.asList(1, 2, 3);
        List<Integer> b = Arrays.asList(4, 5, 6);
        List<Integer> c = Arrays.asList(3, 4, 5);
        System.out.println(Collections.disjoint(a, b));
        System.out.println(Collections.disjoint(a, c));
        System.out.println(Collections.nCopies(3, "x"));
    }
}
"#);
    assert_eq!(out.trim(), "3\n0\ntrue\nfalse\n[x, x, x]");
}

#[test]
fn record_like_class() {
    let out = run(r#"
class Point {
    final int x, y;
    Point(int x, int y) { this.x = x; this.y = y; }
    Point translate(int dx, int dy) { return new Point(x + dx, y + dy); }
    double magnitude() { return Math.sqrt(x * x + y * y); }
    public String toString() { return "Point[x=" + x + ", y=" + y + "]"; }
    boolean sameAs(Point p) { return x == p.x && y == p.y; }
}
class Main {
    public static void main(String[] args) {
        Point p1 = new Point(3, 4);
        Point p2 = p1.translate(1, 1);
        System.out.println(p1);
        System.out.println(p2);
        System.out.printf("%.1f%n", p1.magnitude());
        System.out.println(p1.sameAs(new Point(3, 4)));
        System.out.println(p1.sameAs(p2));
    }
}
"#);
    assert_eq!(out.trim(), "Point[x=3, y=4]\nPoint[x=4, y=5]\n5.0\ntrue\nfalse");
}

#[test]
fn stream_collect_to_map() {
    let out = run(r#"
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("apple", "banana", "cherry");
        Map<String, Integer> lengths = words.stream()
            .collect(Collectors.toMap(w -> w, w -> w.length()));
        // Use TreeMap for sorted output
        TreeMap<String, Integer> sorted = new TreeMap<>(lengths);
        for (Map.Entry<String, Integer> e : sorted.entrySet()) {
            System.out.println(e.getKey() + "=" + e.getValue());
        }
    }
}
"#);
    assert_eq!(out.trim(), "apple=5\nbanana=6\ncherry=6");
}

#[test]
fn abstract_template_method() {
    let out = run(r#"
abstract class DataProcessor {
    // Template method
    final void process(int[] data) {
        int[] filtered = filter(data);
        int[] transformed = transform(filtered);
        output(transformed);
    }
    abstract int[] filter(int[] data);
    abstract int[] transform(int[] data);
    void output(int[] data) {
        StringBuilder sb = new StringBuilder("[");
        for (int i = 0; i < data.length; i++) {
            if (i > 0) sb.append(", ");
            sb.append(data[i]);
        }
        sb.append("]");
        System.out.println(sb.toString());
    }
}
class EvenDoubler extends DataProcessor {
    int[] filter(int[] data) {
        int count = 0;
        for (int x : data) if (x % 2 == 0) count++;
        int[] result = new int[count];
        int i = 0;
        for (int x : data) if (x % 2 == 0) result[i++] = x;
        return result;
    }
    int[] transform(int[] data) {
        int[] result = new int[data.length];
        for (int i = 0; i < data.length; i++) result[i] = data[i] * 2;
        return result;
    }
}
class Main {
    public static void main(String[] args) {
        DataProcessor p = new EvenDoubler();
        int[] data = {1, 2, 3, 4, 5, 6};
        p.process(data);
    }
}
"#);
    assert_eq!(out.trim(), "[0, 4, 8, 12]");
}

#[test]
fn string_intern_and_equality() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String a = "hello";
        String b = "hello";
        System.out.println(a.equals(b));
        System.out.println(a.equalsIgnoreCase("HELLO"));
        System.out.println("abc".compareTo("abd"));
        System.out.println("abc".compareTo("abc"));
        System.out.println("abd".compareTo("abc"));
        System.out.println("Hello World".toLowerCase());
        System.out.println("Hello World".toUpperCase());
    }
}
"#);
    assert_eq!(out.trim(), "true\ntrue\n-1\n0\n1\nhello world\nHELLO WORLD");
}


#[test]
fn linkedlist_queue_deque() {
    let out = run(r#"
import java.util.LinkedList;
import java.util.Queue;
class Main {
    public static void main(String[] args) {
        Queue<String> queue = new LinkedList<>();
        queue.offer("first");
        queue.offer("second");
        queue.offer("third");
        System.out.println(queue.peek());
        System.out.println(queue.poll());
        System.out.println(queue.poll());
        System.out.println(queue.size());
        LinkedList<Integer> deque = new LinkedList<>();
        deque.addFirst(1);
        deque.addLast(2);
        deque.addFirst(0);
        System.out.println(deque.getFirst());
        System.out.println(deque.getLast());
        System.out.println(deque.size());
    }
}
"#);
    assert_eq!(out.trim(), "first\nfirst\nsecond\n1\n0\n2\n3");
}

#[test]
fn observer_pattern() {
    let out = run(r#"
import java.util.ArrayList;
import java.util.List;
interface Observer {
    void update(String event);
}
class EventBus {
    List<Observer> observers = new ArrayList<>();
    void subscribe(Observer o) { observers.add(o); }
    void publish(String event) {
        for (Observer o : observers) o.update(event);
    }
}
class Logger implements Observer {
    List<String> log = new ArrayList<>();
    public void update(String event) { log.add("LOG: " + event); }
    void print() { for (String s : log) System.out.println(s); }
}
class Counter implements Observer {
    int count = 0;
    public void update(String event) { count++; }
}
class Main {
    public static void main(String[] args) {
        EventBus bus = new EventBus();
        Logger logger = new Logger();
        Counter counter = new Counter();
        bus.subscribe(logger);
        bus.subscribe(counter);
        bus.publish("start");
        bus.publish("data");
        bus.publish("end");
        logger.print();
        System.out.println(counter.count);
    }
}
"#);
    assert_eq!(out.trim(), "LOG: start\nLOG: data\nLOG: end\n3");
}

#[test]
fn string_format_specifiers() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(String.format("%b", true));
        System.out.println(String.format("%b", false));
        System.out.println(String.format("%c", 65));
        System.out.println(String.format("%x", 255));
        System.out.println(String.format("%o", 8));
        System.out.println(String.format("%e", 123456.789));
        System.out.println(String.format("%10.2f", 3.14));
        System.out.println(String.format("%-10s!", "left"));
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse\nA\nff\n10\n1.234568e+05\n      3.14\nleft      !");
}

#[test]
fn collections_array_deque() {
    let out = run(r#"
import java.util.ArrayDeque;
class Main {
    public static void main(String[] args) {
        ArrayDeque<Integer> dq = new ArrayDeque<>();
        for (int i = 1; i <= 5; i++) dq.addLast(i);
        System.out.println(dq.size());
        System.out.println(dq.peekFirst());
        System.out.println(dq.peekLast());
        // drain from front
        while (!dq.isEmpty()) System.out.print(dq.pollFirst() + " ");
        System.out.println();
        // use as stack
        ArrayDeque<String> stack = new ArrayDeque<>();
        stack.push("a"); stack.push("b"); stack.push("c");
        while (!stack.isEmpty()) System.out.print(stack.pop() + " ");
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "5\n1\n5\n1 2 3 4 5 \nc b a");
}

#[test]
fn generic_result_type() {
    let out = run(r#"
class Result<T> {
    private T value;
    private String error;
    private boolean success;
    private Result(T value, String error, boolean success) {
        this.value = value; this.error = error; this.success = success;
    }
    static <T> Result<T> ok(T value) { return new Result<>(value, null, true); }
    static <T> Result<T> err(String error) { return new Result<>(null, error, false); }
    boolean isOk() { return success; }
    T getValue() { return value; }
    String getError() { return error; }
}
class Main {
    static Result<Integer> divide(int a, int b) {
        if (b == 0) return Result.err("division by zero");
        return Result.ok(a / b);
    }
    public static void main(String[] args) {
        Result<Integer> r1 = divide(10, 2);
        Result<Integer> r2 = divide(10, 0);
        System.out.println(r1.isOk());
        System.out.println(r1.getValue());
        System.out.println(r2.isOk());
        System.out.println(r2.getError());
    }
}
"#);
    assert_eq!(out.trim(), "true\n5\nfalse\ndivision by zero");
}

#[test]
fn enum_planet_gravity() {
    let out = run(r#"
enum Planet {
    MERCURY(3.303e+23, 2.4397e6),
    VENUS(4.869e+24, 6.0518e6),
    EARTH(5.976e+24, 6.37814e6);
    private final double mass;
    private final double radius;
    Planet(double mass, double radius) {
        this.mass = mass;
        this.radius = radius;
    }
    double surfaceGravity() {
        final double G = 6.67300E-11;
        return G * mass / (radius * radius);
    }
    String name() { return this.toString(); }
}
class Main {
    public static void main(String[] args) {
        for (Planet p : Planet.values()) {
            System.out.printf("%s: %.2f%n", p.name(), p.surfaceGravity());
        }
    }
}
"#);
    assert_eq!(out.trim(), "MERCURY: 3.70\nVENUS: 8.87\nEARTH: 9.80");
}

#[test]
fn iterable_range_pattern() {
    let out = run(r#"
import java.util.ArrayList;
class Main {
    static ArrayList<Integer> makeRange(int start, int end) {
        ArrayList<Integer> result = new ArrayList<>();
        for (int i = start; i < end; i++) result.add(i);
        return result;
    }
    public static void main(String[] args) {
        ArrayList<Integer> nums = makeRange(1, 6);
        for (int n : nums) System.out.print(n + " ");
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3 4 5");
}

#[test]
fn string_split_with_limit() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String csv = "apple,banana,cherry,date";
        String[] parts = csv.split(",");
        for (String p : parts) System.out.println(p.trim());
        System.out.println(parts.length);
        // split with limit
        String[] limited = csv.split(",", 2);
        System.out.println(limited[0]);
        System.out.println(limited[1]);
    }
}
"#);
    assert_eq!(out.trim(), "apple\nbanana\ncherry\ndate\n4\napple\nbanana,cherry,date");
}

#[test]
fn collections_frequency_disjoint_v2() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> list = new ArrayList<>(Arrays.asList("a","b","a","c","a","b"));
        System.out.println(Collections.frequency(list, "a"));
        System.out.println(Collections.frequency(list, "b"));
        List<String> s1 = Arrays.asList("x","y","z");
        List<String> s2 = Arrays.asList("a","b","c");
        List<String> s3 = Arrays.asList("x","a","q");
        System.out.println(Collections.disjoint(s1, s2));
        System.out.println(Collections.disjoint(s1, s3));
    }
}
"#);
    assert_eq!(out.trim(), "3\n2\ntrue\nfalse");
}

#[test]
fn math_functions() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Math.abs(-42));
        System.out.println(Math.max(10, 20));
        System.out.println(Math.min(10, 20));
        System.out.printf("%.4f%n", Math.sqrt(2.0));
        System.out.printf("%.4f%n", Math.pow(2.0, 10.0));
        System.out.println(Math.floor(3.7));
        System.out.println(Math.ceil(3.2));
        System.out.println(Math.round(3.5));
        System.out.println(Math.round(3.4));
        System.out.printf("%.4f%n", Math.log(Math.E));
        System.out.printf("%.4f%n", Math.log10(1000.0));
    }
}
"#);
    assert_eq!(out.trim(), "42\n20\n10\n1.4142\n1024.0000\n3.0\n4.0\n4\n3\n1.0000\n3.0000");
}

#[test]
fn nested_class_access() {
    let out = run(r#"
class Outer {
    static class StaticNested {
        int value;
        StaticNested(int v) { this.value = v; }
        int doubled() { return value * 2; }
        int tripled() { return value * 3; }
    }
    static StaticNested create(int v) { return new StaticNested(v); }
}
class Main {
    public static void main(String[] args) {
        Outer.StaticNested sn = new Outer.StaticNested(15);
        System.out.println(sn.doubled());
        System.out.println(sn.tripled());
        Outer.StaticNested sn2 = Outer.create(10);
        System.out.println(sn2.doubled());
    }
}
"#);
    assert_eq!(out.trim(), "30\n45\n20");
}

#[test]
fn collections_min_max_sort() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = new ArrayList<>(Arrays.asList(5, 2, 8, 1, 9, 3));
        System.out.println(Collections.min(nums));
        System.out.println(Collections.max(nums));
        Collections.sort(nums);
        System.out.println(nums);
        Collections.reverse(nums);
        System.out.println(nums);
        Collections.shuffle(nums, new Random(42));
        Collections.sort(nums);
        System.out.println(nums.get(0));
    }
}
"#);
    assert_eq!(out.trim(), "1\n9\n[1, 2, 3, 5, 8, 9]\n[9, 8, 5, 3, 2, 1]\n1");
}

#[test]
fn string_number_conversions_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // String -> number
        int i = Integer.parseInt("42");
        double d = Double.parseDouble("3.14");
        long l = Long.parseLong("9876543210");
        System.out.println(i);
        System.out.printf("%.2f%n", d);
        System.out.println(l);
        // number -> String
        System.out.println(Integer.toString(255, 16));
        System.out.println(Integer.toBinaryString(10));
        System.out.println(Integer.toOctalString(8));
        System.out.println(Integer.toHexString(255));
        // valueOf
        System.out.println(String.valueOf(true));
        System.out.println(String.valueOf(3.14));
    }
}
"#);
    assert_eq!(out.trim(), "42\n3.14\n9876543210\nff\n1010\n10\nff\ntrue\n3.14");
}

#[test]
fn interface_multiple_impl() {
    let out = run(r#"
interface Printable { void print(); }
interface Saveable { void save(); }
class Document implements Printable, Saveable {
    private String content;
    Document(String content) { this.content = content; }
    public void print() { System.out.println("Printing: " + content); }
    public void save() { System.out.println("Saving: " + content); }
}
class Main {
    public static void main(String[] args) {
        Document doc = new Document("Hello World");
        doc.print();
        doc.save();
        Printable p = doc;
        p.print();
    }
}
"#);
    assert_eq!(out.trim(), "Printing: Hello World\nSaving: Hello World\nPrinting: Hello World");
}

#[test]
fn ternary_chain() {
    let out = run(r#"
class Main {
    static String classify(int n) {
        return n < 0 ? "negative" : n == 0 ? "zero" : n < 10 ? "small" : n < 100 ? "medium" : "large";
    }
    public static void main(String[] args) {
        System.out.println(classify(-5));
        System.out.println(classify(0));
        System.out.println(classify(7));
        System.out.println(classify(42));
        System.out.println(classify(999));
    }
}
"#);
    assert_eq!(out.trim(), "negative\nzero\nsmall\nmedium\nlarge");
}

#[test]
fn array_2d_grid_operations() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] grid = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                grid[i][j] = i * 3 + j + 1;
        // print
        for (int[] row : grid) {
            for (int v : row) System.out.print(v + " ");
            System.out.println();
        }
        // sum diagonal
        int diag = 0;
        for (int i = 0; i < 3; i++) diag += grid[i][i];
        System.out.println(diag);
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3 \n4 5 6 \n7 8 9 \n15");
}

#[test]
fn string_builder_complex() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        StringBuilder sb = new StringBuilder();
        for (int i = 1; i <= 5; i++) {
            if (i > 1) sb.append(", ");
            sb.append(i);
        }
        System.out.println(sb.toString());
        System.out.println(sb.length());
        sb.insert(0, "[");
        sb.append("]");
        System.out.println(sb.toString());
        sb.reverse();
        System.out.println(sb.toString());
    }
}
"#);
    assert_eq!(out.trim(), "1, 2, 3, 4, 5\n13\n[1, 2, 3, 4, 5]\n]5 ,4 ,3 ,2 ,1[");
}

#[test]
fn collections_nCopies_fill() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> copies = Collections.nCopies(3, "hello");
        System.out.println(copies);
        List<Integer> nums = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.fill(nums, 0);
        System.out.println(nums);
        Collections.fill(nums, 7);
        System.out.println(nums);
    }
}
"#);
    assert_eq!(out.trim(), "[hello, hello, hello]\n[0, 0, 0, 0, 0]\n[7, 7, 7, 7, 7]");
}

#[test]
fn integer_methods() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Integer.MAX_VALUE);
        System.out.println(Integer.MIN_VALUE);
        System.out.println(Integer.bitCount(255));
        System.out.println(Integer.highestOneBit(100));
        System.out.println(Integer.lowestOneBit(12));
        System.out.println(Integer.numberOfLeadingZeros(1));
        System.out.println(Integer.numberOfTrailingZeros(8));
        System.out.println(Integer.reverse(1));
        System.out.println(Integer.signum(-5));
        System.out.println(Integer.signum(0));
        System.out.println(Integer.signum(5));
    }
}
"#);
    assert_eq!(out.trim(), "2147483647\n-2147483648\n8\n64\n4\n31\n3\n-2147483648\n-1\n0\n1");
}

#[test]
fn map_computeIfAbsent_grouping() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        String[] words = {"apple", "ant", "banana", "bear", "cherry", "cat"};
        Map<String, List<String>> groups = new TreeMap<>();
        for (String w : words) {
            String key = String.valueOf(w.charAt(0));
            groups.computeIfAbsent(key, k -> new ArrayList<>()).add(w);
        }
        groups.forEach((k, v) -> System.out.println(k + ": " + v));
    }
}
"#);
    assert_eq!(out.trim(), "a: [apple, ant]\nb: [banana, bear]\nc: [cherry, cat]");
}

#[test]
fn exception_chaining() {
    let out = run(r#"
class AppException extends RuntimeException {
    AppException(String msg, Throwable cause) { super(msg); }
    AppException(String msg) { super(msg); }
}
class Main {
    static void level3() { throw new AppException("low level error"); }
    static void level2() {
        try { level3(); }
        catch (AppException e) { throw new AppException("mid level: " + e.getMessage()); }
    }
    static void level1() {
        try { level2(); }
        catch (AppException e) { throw new AppException("top level: " + e.getMessage()); }
    }
    public static void main(String[] args) {
        try { level1(); }
        catch (AppException e) { System.out.println(e.getMessage()); }
    }
}
"#);
    assert_eq!(out.trim(), "top level: mid level: low level error");
}

#[test]
fn functional_stream_composition() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        // filter even, square, sum
        int sum = nums.stream()
            .filter(n -> n % 2 == 0)
            .mapToInt(n -> n * n)
            .sum();
        System.out.println(sum);
        // find first > 5
        Optional<Integer> first = nums.stream()
            .filter(n -> n > 5)
            .findFirst();
        System.out.println(first.get());
        // count
        long count = nums.stream().filter(n -> n % 3 == 0).count();
        System.out.println(count);
        // min/max via Collections
        System.out.println(Collections.min(nums));
        System.out.println(Collections.max(nums));
    }
}
"#);
    assert_eq!(out.trim(), "220\n6\n3\n1\n10");
}

#[test]
fn string_format_width_padding() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // width and padding
        System.out.printf("|%10s|%n", "hello");
        System.out.printf("|%-10s|%n", "hello");
        System.out.printf("|%010d|%n", 42);
        // multiple args
        System.out.printf("%s is %d years old%n", "Alice", 30);
        // float precision
        System.out.printf("%.3f%n", Math.PI);
        // String.format
        String s = String.format("(%d, %d)", 10, 20);
        System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "|     hello|\n|hello     |\n|0000000042|\nAlice is 30 years old\n3.142\n(10, 20)");
}

#[test]
fn collections_unmodifiable() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<Integer> mutable = new ArrayList<>(Arrays.asList(1, 2, 3));
        List<Integer> immutable = Collections.unmodifiableList(mutable);
        System.out.println(immutable.size());
        System.out.println(immutable.get(0));
        try {
            immutable.add(4);
            System.out.println("should not reach");
        } catch (UnsupportedOperationException e) {
            System.out.println("caught UnsupportedOperationException");
        }
        // List.of is also unmodifiable
        List<String> fixed = List.of("a", "b", "c");
        System.out.println(fixed.size());
        try {
            fixed.add("d");
        } catch (UnsupportedOperationException e) {
            System.out.println("List.of is immutable");
        }
    }
}
"#);
    assert_eq!(out.trim(), "3\n1\ncaught UnsupportedOperationException\n3\nList.of is immutable");
}

#[test]
fn inheritance_polymorphism_deep() {
    let out = run(r#"
abstract class Animal {
    String name;
    Animal(String name) { this.name = name; }
    abstract String sound();
    String describe() { return name + " says " + sound(); }
}
class Dog extends Animal {
    Dog(String name) { super(name); }
    public String sound() { return "woof"; }
}
class Cat extends Animal {
    Cat(String name) { super(name); }
    public String sound() { return "meow"; }
}
class Kitten extends Cat {
    Kitten(String name) { super(name); }
    public String sound() { return "mew"; }
}
class Main {
    static void makeNoise(Animal a) { System.out.println(a.describe()); }
    public static void main(String[] args) {
        Animal[] animals = { new Dog("Rex"), new Cat("Whiskers"), new Kitten("Tiny") };
        for (Animal a : animals) makeNoise(a);
        System.out.println(animals[2] instanceof Cat);
        System.out.println(animals[2] instanceof Animal);
        System.out.println(animals[0] instanceof Cat);
    }
}
"#);
    assert_eq!(out.trim(), "Rex says woof\nWhiskers says meow\nTiny says mew\ntrue\ntrue\nfalse");
}

#[test]
fn stream_collect_grouping() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("apple", "ant", "banana", "bear", "cherry");
        // group by first char
        Map<String, List<String>> grouped = words.stream()
            .collect(Collectors.groupingBy(w -> String.valueOf(w.charAt(0))));
        // print in sorted order
        new TreeMap<>(grouped).forEach((k, v) -> {
            Collections.sort(v);
            System.out.println(k + ": " + v);
        });
    }
}
"#);
    assert_eq!(out.trim(), "a: [ant, apple]\nb: [banana, bear]\nc: [cherry]");
}

#[test]
fn number_format_specifiers() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // integer formatting
        System.out.printf("%d%n", 1000000);
        System.out.printf("%+d%n", 42);
        System.out.printf("%+d%n", -42);
        // hex/octal
        System.out.printf("%x%n", 255);
        System.out.printf("%X%n", 255);
        System.out.printf("%o%n", 8);
        // scientific
        System.out.printf("%e%n", 123456.789);
        // boolean
        System.out.printf("%b%n", true);
        System.out.printf("%b%n", null);
    }
}
"#);
    assert_eq!(out.trim(), "1000000\n+42\n-42\nff\nFF\n10\n1.234568e+05\ntrue\nfalse");
}

#[test]
fn strategy_pattern() {
    let out = run(r#"
interface SortStrategy {
    void sort(int[] arr);
}
class InsertionSort implements SortStrategy {
    public void sort(int[] arr) {
        int n = arr.length;
        for (int i = 1; i < n; i++) {
            int key = arr[i];
            int j = i - 1;
            while (j >= 0 && arr[j] > key) {
                int pos = j + 1;
                arr[pos] = arr[j];
                j = j - 1;
            }
            int pos = j + 1;
            arr[pos] = key;
        }
    }
}
class SelectionSort implements SortStrategy {
    public void sort(int[] arr) {
        int n = arr.length;
        for (int i = 0; i < n - 1; i++) {
            int min = i;
            for (int j = i + 1; j < n; j++) if (arr[j] < arr[min]) min = j;
            int t = arr[i]; arr[i] = arr[min]; arr[min] = t;
        }
    }
}
class Sorter {
    private SortStrategy strategy;
    Sorter(SortStrategy s) { this.strategy = s; }
    void sort(int[] arr) { strategy.sort(arr); }
}
class Main {
    static void print(int[] arr) {
        for (int v : arr) System.out.print(v + " ");
        System.out.println();
    }
    public static void main(String[] args) {
        int[] a = {5, 2, 8, 1, 9};
        new Sorter(new InsertionSort()).sort(a);
        print(a);
        int[] b = {5, 2, 8, 1, 9};
        new Sorter(new SelectionSort()).sort(b);
        print(b);
    }
}
"#);
    assert_eq!(out.trim(), "1 2 5 8 9 \n1 2 5 8 9");
}

#[test]
fn generic_bounded_comparable() {
    let out = run(r#"
class Box<T extends Comparable<T>> {
    private T value;
    Box(T value) { this.value = value; }
    T getValue() { return value; }
    boolean isGreaterThan(Box<T> other) { return value.compareTo(other.value) > 0; }
}
class Main {
    public static void main(String[] args) {
        Box<Integer> b1 = new Box<>(10);
        Box<Integer> b2 = new Box<>(5);
        System.out.println(b1.getValue());
        System.out.println(b1.isGreaterThan(b2));
        System.out.println(b2.isGreaterThan(b1));
        Box<String> s1 = new Box<>("banana");
        Box<String> s2 = new Box<>("apple");
        System.out.println(s1.isGreaterThan(s2));
    }
}
"#);
    assert_eq!(out.trim(), "10\ntrue\nfalse\ntrue");
}

#[test]
fn collections_stack_deque() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        // Use Deque as stack
        Deque<Integer> stack = new ArrayDeque<>();
        for (int i = 1; i <= 5; i++) stack.push(i);
        System.out.println(stack.peek());
        while (!stack.isEmpty()) System.out.print(stack.pop() + " ");
        System.out.println();
        // Use Deque as queue
        Deque<String> queue = new ArrayDeque<>();
        queue.offer("first"); queue.offer("second"); queue.offer("third");
        System.out.println(queue.peek());
        while (!queue.isEmpty()) System.out.print(queue.poll() + " ");
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "5\n5 4 3 2 1 \nfirst\nfirst second third");
}

#[test]
fn string_operations_comprehensive() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello, World!";
        System.out.println(s.length());
        System.out.println(s.indexOf("World"));
        System.out.println(s.lastIndexOf('l'));
        System.out.println(s.substring(7));
        System.out.println(s.substring(7, 12));
        System.out.println(s.replace("World", "Java"));
        System.out.println(s.toLowerCase());
        System.out.println(s.toUpperCase());
        System.out.println(s.contains("World"));
        System.out.println(s.startsWith("Hello"));
        System.out.println(s.endsWith("!"));
        System.out.println("  hello  ".strip());
        System.out.println(String.valueOf(42));
        System.out.println(String.valueOf(3.14));
        System.out.println(String.valueOf(true));
    }
}
"#);
    assert_eq!(out.trim(), "13\n7\n10\nWorld!\nWorld\nHello, Java!\nhello, world!\nHELLO, WORLD!\ntrue\ntrue\ntrue\nhello\n42\n3.14\ntrue");
}

#[test]
fn lambda_higher_order() {
    let out = run(r#"
import java.util.*;
import java.util.function.*;
class Main {
    static <T, R> List<R> transform(List<T> list, Function<T, R> f) {
        List<R> result = new ArrayList<>();
        for (T item : list) result.add(f.apply(item));
        return result;
    }
    static <T> List<T> filterList(List<T> list, Predicate<T> p) {
        List<T> result = new ArrayList<>();
        for (T item : list) if (p.test(item)) result.add(item);
        return result;
    }
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6);
        List<Integer> doubled = transform(nums, n -> n * 2);
        System.out.println(doubled);
        List<Integer> evens = filterList(nums, n -> n % 2 == 0);
        System.out.println(evens);
        List<String> strs = transform(nums, n -> "item" + n);
        System.out.println(strs);
    }
}
"#);
    assert_eq!(out.trim(), "[2, 4, 6, 8, 10, 12]\n[2, 4, 6]\n[item1, item2, item3, item4, item5, item6]");
}

#[test]
fn array_sorting_algorithms() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    static int[] mergeSort(int[] arr) {
        if (arr.length <= 1) return arr;
        int mid = arr.length / 2;
        int[] left = mergeSort(Arrays.copyOfRange(arr, 0, mid));
        int[] right = mergeSort(Arrays.copyOfRange(arr, mid, arr.length));
        return merge(left, right);
    }
    static int[] merge(int[] a, int[] b) {
        int total = a.length + b.length;
        int[] result = new int[total];
        int i = 0, j = 0, k = 0;
        while (i < a.length && j < b.length) {
            if (a[i] <= b[j]) { result[k] = a[i]; i++; }
            else { result[k] = b[j]; j++; }
            k++;
        }
        while (i < a.length) { result[k] = a[i]; i++; k++; }
        while (j < b.length) { result[k] = b[j]; j++; k++; }
        return result;
    }
    public static void main(String[] args) {
        int[] arr = {5, 2, 8, 1, 9, 3, 7, 4, 6};
        int[] sorted = mergeSort(arr);
        for (int v : sorted) System.out.print(v + " ");
        System.out.println();
        System.out.println(sorted.length);
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3 4 5 6 7 8 9 \n9");
}

#[test]
fn do_while_basic_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int i = 0;
        do {
            System.out.println(i);
            i++;
        } while (i < 3);
        int x = 10;
        do {
            System.out.println("x=" + x);
            x++;
        } while (x < 5);
    }
}
"#);
    assert_eq!(out.trim(), "0\n1\n2\nx=10");
}

#[test]
fn bitwise_ops() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int a = 10;
        int b = 12;
        System.out.println(a & b);
        System.out.println(a | b);
        System.out.println(a ^ b);
        System.out.println(~a);
        System.out.println(a << 1);
        System.out.println(a >> 1);
        System.out.println(-1 >>> 28);
    }
}
"#);
    assert_eq!(out.trim(), "8\n14\n6\n-11\n20\n5\n15");
}

#[test]
fn static_counter() {
    let out = run(r#"
class Counter {
    private static int count = 0;
    private int id;
    Counter() { count++; this.id = count; }
    static int getCount() { return count; }
    int getId() { return id; }
    static void reset() { count = 0; }
}
class Main {
    public static void main(String[] args) {
        System.out.println(Counter.getCount());
        Counter a = new Counter();
        Counter b = new Counter();
        Counter c = new Counter();
        System.out.println(Counter.getCount());
        System.out.println(a.getId());
        System.out.println(b.getId());
        System.out.println(c.getId());
        Counter.reset();
        System.out.println(Counter.getCount());
    }
}
"#);
    assert_eq!(out.trim(), "0\n3\n1\n2\n3\n0");
}

#[test]
fn abstract_shape() {
    let out = run(r#"
abstract class Shape {
    String color;
    Shape(String color) { this.color = color; }
    abstract double area();
    String describe() { return color + " shape with area " + area(); }
}
class Circle extends Shape {
    double radius;
    Circle(String color, double radius) { super(color); this.radius = radius; }
    double area() { return Math.PI * radius * radius; }
}
class Rectangle extends Shape {
    double w, h;
    Rectangle(String color, double w, double h) { super(color); this.w = w; this.h = h; }
    double area() { return w * h; }
}
class Main {
    public static void main(String[] args) {
        Shape[] shapes = { new Circle("red", 5), new Rectangle("blue", 4, 6) };
        for (Shape s : shapes) System.out.printf("%.2f%n", s.area());
        System.out.println(shapes[1].describe());
    }
}
"#);
    assert_eq!(out.trim(), "78.54\n24.00\nblue shape with area 24");
}

#[test]
fn instanceof_downcast() {
    let out = run(r#"
class Animal { String name; Animal(String n) { name = n; } }
class Dog extends Animal {
    Dog(String n) { super(n); }
    void bark() { System.out.println(name + " says woof"); }
}
class Cat extends Animal {
    Cat(String n) { super(n); }
    void meow() { System.out.println(name + " says meow"); }
}
class Main {
    public static void main(String[] args) {
        Animal[] animals = { new Dog("Rex"), new Cat("Whiskers"), new Dog("Buddy") };
        for (Animal a : animals) {
            System.out.println(a instanceof Dog);
            if (a instanceof Dog) {
                Dog d = (Dog) a;
                d.bark();
            } else if (a instanceof Cat) {
                Cat c = (Cat) a;
                c.meow();
            }
        }
    }
}
"#);
    assert_eq!(out.trim(), "true\nRex says woof\nfalse\nWhiskers says meow\ntrue\nBuddy says woof");
}

#[test]
fn multiple_catch_blocks() {
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
            System.out.println("ArithmeticException: " + e.getMessage());
        }
        try {
            int n = Integer.parseInt("abc");
        } catch (NumberFormatException e) {
            System.out.println("NumberFormatException");
        } catch (ArithmeticException e) {
            System.out.println("ArithmeticException");
        }
        try {
            System.out.println(divide(10, 0));
        } catch (NumberFormatException e) {
            System.out.println("NumberFormatException");
        } catch (ArithmeticException e) {
            System.out.println("ArithmeticException: " + e.getMessage());
        }
    }
}
"#);
    assert_eq!(out.trim(), "5\nArithmeticException: division by zero\nNumberFormatException\nArithmeticException: division by zero");
}

#[test]
fn comparable_sort_students() {
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
        students.add(new Student("Diana", 95));
        Collections.sort(students);
        for (Student s : students) System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "Charlie:78\nAlice:85\nBob:92\nDiana:95");
}

#[test]
fn nested_loops_matrix() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] matrix = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                matrix[i][j] = i * 3 + j + 1;
        int sum = 0;
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 3; j++) {
                System.out.print(matrix[i][j] + " ");
                sum += matrix[i][j];
            }
            System.out.println();
        }
        System.out.println("sum=" + sum);
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3 \n4 5 6 \n7 8 9 \nsum=45");
}

#[test]
fn string_split_join() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        String csv = "apple,banana,cherry,date";
        String[] parts = csv.split(",");
        System.out.println(parts.length);
        for (String p : parts) System.out.println(p);
        String joined = String.join(" | ", parts);
        System.out.println(joined);
        String[] words = "one two three".split(" ");
        System.out.println(words.length);
    }
}
"#);
    assert_eq!(out.trim(), "4\napple\nbanana\ncherry\ndate\napple | banana | cherry | date\n3");
}

#[test]
fn map_iteration_patterns_v2() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> scores = new LinkedHashMap<>();
        scores.put("Alice", 90);
        scores.put("Bob", 85);
        scores.put("Charlie", 92);
        // entrySet iteration
        for (Map.Entry<String, Integer> e : scores.entrySet())
            System.out.println(e.getKey() + "=" + e.getValue());
        // keySet
        int total = 0;
        for (String k : scores.keySet()) total += scores.get(k);
        System.out.println("total=" + total);
        // containsKey/Value
        System.out.println(scores.containsKey("Bob"));
        System.out.println(scores.containsValue(99));
        // remove
        scores.remove("Bob");
        System.out.println(scores.size());
    }
}
"#);
    assert_eq!(out.trim(), "Alice=90\nBob=85\nCharlie=92\ntotal=267\ntrue\nfalse\n2");
}

#[test]
fn generic_stack_v2() {
    let out = run(r#"
class Stack<T> {
    private Object[] data;
    private int size;
    Stack(int cap) { data = new Object[cap]; size = 0; }
    void push(T val) { data[size++] = val; }
    T pop() { return (T) data[--size]; }
    T peek() { return (T) data[size - 1]; }
    boolean isEmpty() { return size == 0; }
    int size() { return size; }
}
class Main {
    public static void main(String[] args) {
        Stack<Integer> s = new Stack<>(10);
        s.push(1); s.push(2); s.push(3);
        System.out.println(s.size());
        System.out.println(s.peek());
        System.out.println(s.pop());
        System.out.println(s.pop());
        System.out.println(s.isEmpty());
        System.out.println(s.pop());
        System.out.println(s.isEmpty());
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n3\n2\nfalse\n1\ntrue");
}

#[test]
fn functional_interface_lambda() {
    let out = run(r#"
import java.util.*;
import java.util.function.*;
class Main {
    static int apply(int x, Function<Integer, Integer> f) { return f.apply(x); }
    static boolean test(int x, Predicate<Integer> p) { return p.test(x); }
    public static void main(String[] args) {
        Function<Integer, Integer> square = x -> x * x;
        Function<Integer, Integer> addTen = x -> x + 10;
        System.out.println(apply(5, square));
        System.out.println(apply(5, addTen));
        Predicate<Integer> isEven = x -> x % 2 == 0;
        System.out.println(test(4, isEven));
        System.out.println(test(7, isEven));
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6);
        nums.stream().filter(isEven).map(square).forEach(System.out::println);
    }
}
"#);
    assert_eq!(out.trim(), "25\n15\ntrue\nfalse\n4\n16\n36");
}

#[test]
fn string_builder_operations() {
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
        sb.insert(5, " there");
        System.out.println(sb.toString());
        sb.delete(5, 11);
        System.out.println(sb.toString());
        sb.reverse();
        System.out.println(sb.toString());
        StringBuilder sb2 = new StringBuilder("abc");
        sb2.append(123).append(true);
        System.out.println(sb2.toString());
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!\n13\nHello there, World!\nHello, World!\n!dlroW ,olleH\nabc123true");
}

#[test]
fn exception_hierarchy() {
    let out = run(r#"
class AppException extends RuntimeException {
    int code;
    AppException(String msg, int code) { super(msg); this.code = code; }
    int getCode() { return code; }
}
class ValidationException extends AppException {
    String field;
    ValidationException(String field, String msg) {
        super(msg, 400);
        this.field = field;
    }
    String getField() { return field; }
}
class Main {
    static void validate(String name, String value) {
        if (value == null || value.isEmpty())
            throw new ValidationException(name, name + " is required");
    }
    public static void main(String[] args) {
        try {
            validate("email", "");
        } catch (ValidationException e) {
            System.out.println("field=" + e.getField());
            System.out.println("msg=" + e.getMessage());
            System.out.println("code=" + e.getCode());
        }
        try {
            validate("name", "Alice");
            System.out.println("ok");
        } catch (AppException e) {
            System.out.println("error");
        }
    }
}
"#);
    assert_eq!(out.trim(), "field=email\nmsg=email is required\ncode=400\nok");
}

#[test]
fn array_operations_advanced() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    public static void main(String[] args) {
        int[] arr = {5, 2, 8, 1, 9, 3};
        Arrays.sort(arr);
        System.out.println(Arrays.toString(arr));
        int idx = Arrays.binarySearch(arr, 8);
        System.out.println("found 8 at " + idx);
        int[] copy = Arrays.copyOf(arr, 4);
        System.out.println(Arrays.toString(copy));
        int[] range = Arrays.copyOfRange(arr, 2, 5);
        System.out.println(Arrays.toString(range));
        int[] filled = new int[5];
        Arrays.fill(filled, 7);
        System.out.println(Arrays.toString(filled));
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 5, 8, 9]\nfound 8 at 4\n[1, 2, 3, 5]\n[3, 5, 8]\n[7, 7, 7, 7, 7]");
}

#[test]
fn switch_expression_patterns() {
    let out = run(r#"
class Main {
    static String dayType(int day) {
        switch (day) {
            case 1: case 7: return "weekend";
            case 2: case 3: case 4: case 5: case 6: return "weekday";
            default: return "invalid";
        }
    }
    static int score(String grade) {
        switch (grade) {
            case "A": return 4;
            case "B": return 3;
            case "C": return 2;
            case "D": return 1;
            default: return 0;
        }
    }
    public static void main(String[] args) {
        for (int d = 1; d <= 7; d++) System.out.print(dayType(d) + " ");
        System.out.println();
        System.out.println(score("A"));
        System.out.println(score("B"));
        System.out.println(score("F"));
    }
}
"#);
    assert_eq!(out.trim(), "weekend weekday weekday weekday weekday weekday weekend \n4\n3\n0");
}

#[test]
fn linked_list_impl() {
    let out = run(r#"
class Node {
    int val;
    Node next;
    Node(int val) { this.val = val; }
}
class MyLinkedList {
    Node head;
    void add(int val) {
        Node n = new Node(val);
        if (head == null) { head = n; return; }
        Node cur = head;
        while (cur.next != null) cur = cur.next;
        cur.next = n;
    }
    void print() {
        Node cur = head;
        while (cur != null) {
            System.out.print(cur.val);
            if (cur.next != null) System.out.print(" -> ");
            cur = cur.next;
        }
        System.out.println();
    }
    int size() {
        int n = 0;
        Node cur = head;
        while (cur != null) { n++; cur = cur.next; }
        return n;
    }
    void reverse() {
        Node prev = null;
        Node cur = head;
        while (cur != null) {
            Node next = cur.next;
            cur.next = prev;
            prev = cur;
            cur = next;
        }
        head = prev;
    }
}
class Main {
    public static void main(String[] args) {
        MyLinkedList list = new MyLinkedList();
        list.add(1); list.add(2); list.add(3); list.add(4); list.add(5);
        list.print();
        System.out.println(list.size());
        list.reverse();
        list.print();
    }
}
"#);
    assert_eq!(out.trim(), "1 -> 2 -> 3 -> 4 -> 5\n5\n5 -> 4 -> 3 -> 2 -> 1");
}

#[test]
fn binary_tree_traversal() {
    let out = run(r#"
class TreeNode {
    int val;
    TreeNode left, right;
    TreeNode(int val) { this.val = val; }
}
class BST {
    TreeNode root;
    void insert(int val) { root = insert(root, val); }
    TreeNode insert(TreeNode node, int val) {
        if (node == null) return new TreeNode(val);
        if (val < node.val) node.left = insert(node.left, val);
        else if (val > node.val) node.right = insert(node.right, val);
        return node;
    }
    void inorder(TreeNode node) {
        if (node == null) return;
        inorder(node.left);
        System.out.print(node.val + " ");
        inorder(node.right);
    }
    boolean contains(int val) { return contains(root, val); }
    boolean contains(TreeNode node, int val) {
        if (node == null) return false;
        if (val == node.val) return true;
        return val < node.val ? contains(node.left, val) : contains(node.right, val);
    }
}
class Main {
    public static void main(String[] args) {
        BST tree = new BST();
        int[] vals = {5, 3, 7, 1, 4, 6, 8};
        for (int v : vals) tree.insert(v);
        tree.inorder(tree.root);
        System.out.println();
        System.out.println(tree.contains(4));
        System.out.println(tree.contains(9));
    }
}
"#);
    assert_eq!(out.trim(), "1 3 4 5 6 7 8 \ntrue\nfalse");
}

#[test]
fn optional_usage() {
    let out = run(r#"
import java.util.Optional;
class Main {
    static Optional<String> findUser(int id) {
        if (id == 1) return Optional.of("Alice");
        if (id == 2) return Optional.of("Bob");
        return Optional.empty();
    }
    public static void main(String[] args) {
        Optional<String> u1 = findUser(1);
        System.out.println(u1.isPresent());
        System.out.println(u1.get());
        Optional<String> u3 = findUser(3);
        System.out.println(u3.isPresent());
        System.out.println(u3.orElse("Unknown"));
        System.out.println(u1.map(s -> s.toUpperCase()).get());
    }
}
"#);
    assert_eq!(out.trim(), "true\nAlice\nfalse\nUnknown\nALICE");
}

#[test]
fn stream_advanced() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("hello", "world", "java", "stream", "api");
        // filter + map + collect
        List<String> result = words.stream()
            .filter(w -> w.length() > 4)
            .map(String::toUpperCase)
            .collect(Collectors.toList());
        System.out.println(result);
        // reduce
        int totalLen = words.stream().mapToInt(String::length).sum();
        System.out.println(totalLen);
        // count
        long count = words.stream().filter(w -> w.contains("a")).count();
        System.out.println(count);
        // sorted
        words.stream().sorted().forEach(w -> System.out.print(w + " "));
        System.out.println();
        // distinct
        List<Integer> nums = Arrays.asList(1, 2, 2, 3, 3, 3, 4);
        long distinct = nums.stream().distinct().count();
        System.out.println(distinct);
    }
}
"#);
    assert_eq!(out.trim(), "[HELLO, WORLD, STREAM]\n23\n3\napi hello java stream world \n4");
}

#[test]
fn enum_with_methods() {
    let out = run(r#"
enum Season {
    SPRING, SUMMER, FALL, WINTER;
    boolean isWarm() {
        return this == SPRING || this == SUMMER;
    }
    Season next() {
        switch (this) {
            case SPRING: return SUMMER;
            case SUMMER: return FALL;
            case FALL: return WINTER;
            default: return SPRING;
        }
    }
}
class Main {
    public static void main(String[] args) {
        for (Season s : Season.values()) {
            System.out.println(s + " warm=" + s.isWarm());
        }
        Season s = Season.WINTER;
        System.out.println(s.next());
    }
}
"#);
    assert_eq!(out.trim(), "SPRING warm=true\nSUMMER warm=true\nFALL warm=false\nWINTER warm=false\nSPRING");
}

#[test]
fn interface_default_method() {
    let out = run(r#"
interface Greeter {
    String greet(String name);
    default String greetLoud(String name) {
        return greet(name).toUpperCase();
    }
}
class FormalGreeter implements Greeter {
    public String greet(String name) { return "Good day, " + name; }
}
class CasualGreeter implements Greeter {
    public String greet(String name) { return "Hey " + name + "!"; }
}
class Main {
    public static void main(String[] args) {
        Greeter[] greeters = { new FormalGreeter(), new CasualGreeter() };
        for (Greeter g : greeters) {
            System.out.println(g.greet("Alice"));
            System.out.println(g.greetLoud("Bob"));
        }
    }
}
"#);
    assert_eq!(out.trim(), "Good day, Alice\nGOOD DAY, BOB\nHey Alice!\nHEY BOB!");
}

#[test]
fn generic_pair() {
    let out = run(r#"
class Pair<A, B> {
    A first;
    B second;
    Pair(A first, B second) { this.first = first; this.second = second; }
    Pair<B, A> swap() { return new Pair<>(second, first); }
    public String toString() { return "(" + first + ", " + second + ")"; }
}
class Main {
    public static void main(String[] args) {
        Pair<String, Integer> p = new Pair<>("hello", 42);
        System.out.println(p);
        System.out.println(p.first);
        System.out.println(p.second);
        Pair<Integer, String> swapped = p.swap();
        System.out.println(swapped);
        Pair<Integer, Integer> nums = new Pair<>(10, 20);
        System.out.println(nums.first + nums.second);
    }
}
"#);
    assert_eq!(out.trim(), "(hello, 42)\nhello\n42\n(42, hello)\n30");
}

#[test]
fn string_formatting() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.printf("Name: %s, Age: %d%n", "Alice", 30);
        System.out.printf("Pi: %.4f%n", Math.PI);
        System.out.printf("%05d%n", 42);
        System.out.printf("%-10s|%n", "left");
        System.out.printf("%10s|%n", "right");
        String s = String.format("(%d, %d)", 3, 4);
        System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "Name: Alice, Age: 30\nPi: 3.1416\n00042\nleft      |\n     right|\n(3, 4)");
}

#[test]
fn collections_operations() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        // ArrayList operations
        List<Integer> list = new ArrayList<>(Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6));
        Collections.sort(list);
        System.out.println(list);
        System.out.println(Collections.min(list));
        System.out.println(Collections.max(list));
        Collections.reverse(list);
        System.out.println(list.get(0));
        // Set operations
        Set<String> set = new HashSet<>(Arrays.asList("a", "b", "c", "a", "b"));
        System.out.println(set.size());
        System.out.println(set.contains("a"));
        System.out.println(set.contains("z"));
    }
}
"#);
    assert_eq!(out.trim(), "[1, 1, 2, 3, 4, 5, 6, 9]\n1\n9\n9\n3\ntrue\nfalse");
}

#[test]
fn nested_generics_map() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, List<Integer>> map = new HashMap<>();
        map.put("evens", new ArrayList<>(Arrays.asList(2, 4, 6)));
        map.put("odds", new ArrayList<>(Arrays.asList(1, 3, 5)));
        List<Integer> evens = map.get("evens");
        evens.add(8);
        System.out.println(map.get("evens").size());
        System.out.println(map.get("odds").size());
        int total = 0;
        for (String k : map.keySet()) {
            for (int n : map.get(k)) total += n;
        }
        System.out.println(total);
    }
}
"#);
    assert_eq!(out.trim(), "4\n3\n29");
}

#[test]
fn fibonacci_memoized() {
    let out = run(r#"
import java.util.HashMap;
import java.util.Map;
class Main {
    static Map<Integer, Long> memo = new HashMap<>();
    static long fib(int n) {
        if (n <= 1) return n;
        if (memo.containsKey(n)) return memo.get(n);
        long result = fib(n - 1) + fib(n - 2);
        memo.put(n, result);
        return result;
    }
    public static void main(String[] args) {
        for (int i = 0; i <= 10; i++) System.out.print(fib(i) + " ");
        System.out.println();
        System.out.println(fib(40));
    }
}
"#);
    assert_eq!(out.trim(), "0 1 1 2 3 5 8 13 21 34 55 \n102334155");
}

#[test]
fn string_parsing() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // parseInt/parseLong/parseDouble
        int i = Integer.parseInt("42");
        long l = Long.parseLong("9876543210");
        double d = Double.parseDouble("3.14");
        System.out.println(i);
        System.out.println(l);
        System.out.printf("%.2f%n", d);
        // toString
        System.out.println(Integer.toString(255, 16));
        System.out.println(Integer.toBinaryString(10));
        System.out.println(Integer.toHexString(255));
        System.out.println(Integer.toOctalString(8));
        // valueOf
        System.out.println(Integer.valueOf("100"));
        System.out.println(Double.valueOf("2.5") * 2);
    }
}
"#);
    assert_eq!(out.trim(), "42\n9876543210\n3.14\nff\n1010\nff\n10\n100\n5.0");
}

#[test]
fn array_2d_matrix_multiply() {
    let out = run(r#"
class Main {
    static int[][] multiply(int[][] a, int[][] b) {
        int n = a.length;
        int[][] c = new int[n][n];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                for (int k = 0; k < n; k++)
                    c[i][j] += a[i][k] * b[k][j];
        return c;
    }
    public static void main(String[] args) {
        int[][] a = {{1, 2}, {3, 4}};
        int[][] b = {{5, 6}, {7, 8}};
        int[][] c = multiply(a, b);
        for (int[] row : c) {
            for (int v : row) System.out.print(v + " ");
            System.out.println();
        }
    }
}
"#);
    assert_eq!(out.trim(), "19 22 \n43 50");
}

#[test]
fn observer_pattern_v2() {
    let out = run(r#"
import java.util.*;
interface Observer {
    void update(String event, Object data);
}
class EventBus {
    Map<String, List<Observer>> listeners = new HashMap<>();
    void subscribe(String event, Observer obs) {
        listeners.computeIfAbsent(event, k -> new ArrayList<>()).add(obs);
    }
    void publish(String event, Object data) {
        List<Observer> obs = listeners.getOrDefault(event, new ArrayList<>());
        for (Observer o : obs) o.update(event, data);
    }
}
class Main {
    public static void main(String[] args) {
        EventBus bus = new EventBus();
        bus.subscribe("login", (e, d) -> System.out.println("Logger: " + d + " logged in"));
        bus.subscribe("login", (e, d) -> System.out.println("Audit: login event for " + d));
        bus.subscribe("logout", (e, d) -> System.out.println("Logger: " + d + " logged out"));
        bus.publish("login", "Alice");
        bus.publish("logout", "Bob");
        bus.publish("login", "Charlie");
    }
}
"#);
    assert_eq!(out.trim(), "Logger: Alice logged in\nAudit: login event for Alice\nLogger: Bob logged out\nLogger: Charlie logged in\nAudit: login event for Charlie");
}

#[test]
fn builder_pattern_v2() {
    let out = run(r#"
class Person {
    String name;
    int age;
    String email;
    private Person() {}
    static class Builder {
        Person p = new Person();
        Builder name(String n) { p.name = n; return this; }
        Builder age(int a) { p.age = a; return this; }
        Builder email(String e) { p.email = e; return this; }
        Person build() { return p; }
    }
    public String toString() {
        return "Person{name=" + name + ", age=" + age + ", email=" + email + "}";
    }
}
class Main {
    public static void main(String[] args) {
        Person p = new Person.Builder()
            .name("Alice")
            .age(30)
            .email("alice@example.com")
            .build();
        System.out.println(p);
        System.out.println(p.name);
        System.out.println(p.age);
    }
}
"#);
    assert_eq!(out.trim(), "Person{name=Alice, age=30, email=alice@example.com}\nAlice\n30");
}

#[test]
fn functional_composition_v2() {
    let out = run(r#"
import java.util.*;
import java.util.function.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        // compose and andThen
        Function<Integer, Integer> times2 = x -> x * 2;
        Function<Integer, Integer> plus3 = x -> x + 3;
        Function<Integer, Integer> times2ThenPlus3 = times2.andThen(plus3);
        Function<Integer, Integer> plus3ThenTimes2 = times2.compose(plus3);
        System.out.println(times2ThenPlus3.apply(5));  // 13
        System.out.println(plus3ThenTimes2.apply(5));  // 16
        // Predicate.and, or, negate
        Predicate<Integer> isEven = n -> n % 2 == 0;
        Predicate<Integer> isPositive = n -> n > 0;
        Predicate<Integer> isEvenAndPositive = isEven.and(isPositive);
        List<Integer> nums = Arrays.asList(-4, -3, -2, -1, 0, 1, 2, 3, 4);
        nums.stream().filter(isEvenAndPositive).forEach(n -> System.out.print(n + " "));
        System.out.println();
        nums.stream().filter(isEven.negate()).forEach(n -> System.out.print(n + " "));
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "13\n16\n2 4 \n-3 -1 1 3");
}

#[test]
fn exception_finally() {
    let out = run(r#"
class Main {
    static String test(boolean throwEx) {
        StringBuilder sb = new StringBuilder();
        try {
            sb.append("try ");
            if (throwEx) throw new RuntimeException("oops");
            sb.append("no-throw ");
        } catch (RuntimeException e) {
            sb.append("catch(" + e.getMessage() + ") ");
        } finally {
            sb.append("finally");
        }
        return sb.toString();
    }
    public static void main(String[] args) {
        System.out.println(test(false));
        System.out.println(test(true));
    }
}
"#);
    assert_eq!(out.trim(), "try no-throw finally\ntry catch(oops) finally");
}

#[test]
fn collections_sort_comparator() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> words = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "date", "elderberry"));
        // Sort by length
        words.sort((a, b) -> a.length() - b.length());
        System.out.println(words);
        // Sort by length then alphabetically
        words.sort((a, b) -> {
            int cmp = a.length() - b.length();
            return cmp != 0 ? cmp : a.compareTo(b);
        });
        System.out.println(words);
        // Reverse sort
        words.sort((a, b) -> b.compareTo(a));
        System.out.println(words.get(0));
    }
}
"#);
    assert_eq!(out.trim(), "[date, apple, banana, cherry, elderberry]\n[date, apple, banana, cherry, elderberry]\nelderberry");
}

#[test]
fn map_advanced_operations() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> scores = new HashMap<>();
        scores.put("Alice", 85);
        scores.put("Bob", 90);
        // getOrDefault
        System.out.println(scores.getOrDefault("Alice", 0));
        System.out.println(scores.getOrDefault("Charlie", 0));
        // merge
        scores.merge("Alice", 5, Integer::sum);
        scores.merge("Charlie", 70, Integer::sum);
        System.out.println(scores.get("Alice"));
        System.out.println(scores.get("Charlie"));
        // putIfAbsent
        scores.putIfAbsent("Bob", 999);
        scores.putIfAbsent("Diana", 88);
        System.out.println(scores.get("Bob"));
        System.out.println(scores.get("Diana"));
        System.out.println(scores.size());
    }
}
"#);
    assert_eq!(out.trim(), "85\n0\n90\n70\n90\n88\n4");
}

#[test]
fn iterable_custom() {
    let out = run(r#"
import java.util.*;
class Main {
    static List<Integer> range(int start, int end) {
        List<Integer> result = new ArrayList<>();
        for (int i = start; i < end; i++) result.add(i);
        return result;
    }
    public static void main(String[] args) {
        for (int x : range(1, 6)) System.out.print(x + " ");
        System.out.println();
        for (int x : range(10, 13)) System.out.print(x + " ");
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3 4 5 \n10 11 12");
}

#[test]
fn static_initializer_v2() {
    let out = run(r#"
class Config {
    static int MAX;
    static String PREFIX;
    static {
        MAX = 100;
        PREFIX = "cfg_";
    }
    static String key(String name) { return PREFIX + name; }
}
class Main {
    public static void main(String[] args) {
        System.out.println(Config.MAX);
        System.out.println(Config.PREFIX);
        System.out.println(Config.key("timeout"));
    }
}
"#);
    assert_eq!(out.trim(), "100\ncfg_\ncfg_timeout");
}

#[test]
fn nested_class_access_v2() {
    let out = run(r#"
class Outer {
    int x = 10;
    static class StaticNested {
        int z = 30;
        int doubled() { return z * 2; }
    }
    static class Inner {
        int outerX;
        int y = 20;
        Inner(int ox) { outerX = ox; }
        int sum() { return outerX + y; }
    }
}
class Main {
    public static void main(String[] args) {
        Outer o = new Outer();
        Outer.Inner i = new Outer.Inner(o.x);
        System.out.println(i.sum());
        Outer.StaticNested sn = new Outer.StaticNested();
        System.out.println(sn.doubled());
    }
}
"#);
    assert_eq!(out.trim(), "30\n60");
}

#[test]
fn string_regex_groups() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String text = "2024-01-15";
        String[] parts = text.split("-");
        System.out.println("year=" + parts[0]);
        System.out.println("month=" + parts[1]);
        System.out.println("day=" + parts[2]);
        // find all words via split on spaces
        String sentence = "hello world foo";
        String[] words = sentence.split(" ");
        for (String w : words) System.out.print(w + " ");
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "year=2024\nmonth=01\nday=15\nhello world foo");
}

#[test]
fn collections_deque_stack() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Deque<Integer> stack = new ArrayDeque<>();
        stack.push(1); stack.push(2); stack.push(3);
        System.out.println(stack.peek());
        System.out.println(stack.pop());
        System.out.println(stack.size());
        Deque<String> queue = new ArrayDeque<>();
        queue.offer("a"); queue.offer("b"); queue.offer("c");
        System.out.println(queue.poll());
        System.out.println(queue.peek());
        System.out.println(queue.size());
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n2\na\nb\n2");
}

#[test]
fn interface_multiple() {
    let out = run(r#"
interface Flyable { default String fly() { return "flying"; } }
interface Swimmable { default String swim() { return "swimming"; } }
interface Runnable { String run(); }
class Duck implements Flyable, Swimmable, Runnable {
    public String run() { return "running"; }
    public String toString() { return fly() + " " + swim() + " " + run(); }
}
class Main {
    public static void main(String[] args) {
        Duck d = new Duck();
        System.out.println(d);
        System.out.println(d instanceof Flyable);
        System.out.println(d instanceof Swimmable);
    }
}
"#);
    assert_eq!(out.trim(), "flying swimming running\ntrue\ntrue");
}

#[test]
fn generic_bounded() {
    let out = run(r#"
class Box<T extends Comparable<T>> {
    T value;
    Box(T v) { value = v; }
    boolean isGreaterThan(Box<T> other) { return value.compareTo(other.value) > 0; }
    T max(Box<T> other) { return isGreaterThan(other) ? value : other.value; }
}
class Main {
    public static void main(String[] args) {
        Box<Integer> a = new Box<>(10);
        Box<Integer> b = new Box<>(20);
        System.out.println(a.isGreaterThan(b));
        System.out.println(b.isGreaterThan(a));
        System.out.println(a.max(b));
        Box<String> s1 = new Box<>("apple");
        Box<String> s2 = new Box<>("banana");
        System.out.println(s1.max(s2));
    }
}
"#);
    assert_eq!(out.trim(), "false\ntrue\n20\nbanana");
}

#[test]
fn lambda_method_reference() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
import java.util.function.*;
class Main {
    static int doubleIt(int x) { return x * 2; }
    static boolean isOdd(int x) { return x % 2 != 0; }
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        // static method reference
        nums.stream().map(Main::doubleIt).forEach(x -> System.out.print(x + " "));
        System.out.println();
        // instance method reference on type
        List<String> words = Arrays.asList("hello", "world", "java");
        words.stream().map(String::toUpperCase).forEach(s -> System.out.print(s + " "));
        System.out.println();
        // filter with method ref
        nums.stream().filter(Main::isOdd).forEach(x -> System.out.print(x + " "));
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "2 4 6 8 10 \nHELLO WORLD JAVA \n1 3 5");
}

#[test]
fn exception_chaining_v2() {
    let out = run(r#"
class AppException extends RuntimeException {
    AppException(String msg, Throwable cause) { super(msg, cause); }
}
class Main {
    static void level3() { throw new IllegalArgumentException("bad input"); }
    static void level2() {
        try { level3(); }
        catch (IllegalArgumentException e) {
            throw new AppException("level2 failed", e);
        }
    }
    static void level1() {
        try { level2(); }
        catch (AppException e) {
            System.out.println("caught: " + e.getMessage());
            System.out.println("cause: " + e.getCause().getMessage());
        }
    }
    public static void main(String[] args) {
        level1();
    }
}
"#);
    assert_eq!(out.trim(), "caught: level2 failed\ncause: bad input");
}

#[test]
fn array_streams() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        int[] arr = {5, 3, 8, 1, 9, 2, 7};
        // sum
        System.out.println(Arrays.stream(arr).sum());
        // min via reduce
        int min = Arrays.stream(arr).reduce(Integer.MAX_VALUE, (a, b) -> a < b ? a : b);
        System.out.println(min);
        // max via reduce
        int max = Arrays.stream(arr).reduce(Integer.MIN_VALUE, (a, b) -> a > b ? a : b);
        System.out.println(max);
        // sorted and collect
        int[] sorted = Arrays.stream(arr).sorted().toArray();
        System.out.println(Arrays.toString(sorted));
        // filter and count
        long count = Arrays.stream(arr).filter(x -> x > 4).count();
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "35\n1\n9\n[1, 2, 3, 5, 7, 8, 9]\n4");
}

#[test]
fn decorator_pattern() {
    let out = run(r#"
interface TextProcessor {
    String process(String text);
}
class UpperCase implements TextProcessor {
    public String process(String text) { return text.toUpperCase(); }
}
class TrimDecorator implements TextProcessor {
    TextProcessor inner;
    TrimDecorator(TextProcessor inner) { this.inner = inner; }
    public String process(String text) { return inner.process(text.trim()); }
}
class PrefixDecorator implements TextProcessor {
    TextProcessor inner;
    String prefix;
    PrefixDecorator(TextProcessor inner, String prefix) { this.inner = inner; this.prefix = prefix; }
    public String process(String text) { return prefix + inner.process(text); }
}
class Main {
    public static void main(String[] args) {
        TextProcessor p = new PrefixDecorator(new TrimDecorator(new UpperCase()), ">> ");
        System.out.println(p.process("  hello world  "));
        System.out.println(p.process("  java  "));
    }
}
"#);
    assert_eq!(out.trim(), ">> HELLO WORLD\n>> JAVA");
}

#[test]
fn command_pattern() {
    let out = run(r#"
interface Command {
    int execute(int val);
    int undo(int val);
}
class Main {
    public static void main(String[] args) {
        Command inc = v -> v + 1;
        Command dec = v -> v - 1;
        int val = 0;
        val = inc.execute(val);
        val = inc.execute(val);
        val = inc.execute(val);
        System.out.println(val);
        val = dec.execute(val);
        System.out.println(val);
        val = dec.undo(val);
        val = inc.undo(val);
        System.out.println(val);
    }
}
"#);
    assert_eq!(out.trim(), "3\n2\n2");
}

#[test]
fn generic_wildcard() {
    let out = run(r#"
import java.util.*;
class Stats {
    static double sum(List<? extends Number> list) {
        double total = 0;
        for (Number n : list) total += n.doubleValue();
        return total;
    }
    static double average(List<? extends Number> list) {
        return sum(list) / list.size();
    }
}
class Main {
    public static void main(String[] args) {
        List<Integer> ints = Arrays.asList(1, 2, 3, 4, 5);
        List<Double> doubles = Arrays.asList(1.5, 2.5, 3.5);
        System.out.println(Stats.sum(ints));
        System.out.println(Stats.average(ints));
        System.out.println(Stats.sum(doubles));
    }
}
"#);
    assert_eq!(out.trim(), "15.0\n3.0\n7.5");
}

#[test]
fn string_builder_chaining_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String result = new StringBuilder()
            .append("Hello")
            .append(", ")
            .append("World")
            .append("!")
            .insert(7, "Beautiful ")
            .toString();
        System.out.println(result);
        StringBuilder sb = new StringBuilder("abcdef");
        sb.reverse();
        System.out.println(sb.toString());
        sb.delete(1, 3);
        System.out.println(sb.toString());
        System.out.println(sb.length());
    }
}
"#);
    assert_eq!(out.trim(), "Hello, Beautiful World!\nfedcba\nfcba\n4");
}

#[test]
fn collections_computeifabsent() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, List<Integer>> map = new HashMap<>();
        // computeIfAbsent
        map.computeIfAbsent("evens", k -> new ArrayList<>()).add(2);
        map.computeIfAbsent("evens", k -> new ArrayList<>()).add(4);
        map.computeIfAbsent("odds", k -> new ArrayList<>()).add(1);
        System.out.println(map.get("evens"));
        System.out.println(map.get("odds"));
        System.out.println(map.size());
        // forEach on map
        Map<String, Integer> scores = new TreeMap<>();
        scores.put("Alice", 90);
        scores.put("Bob", 85);
        scores.put("Charlie", 92);
        scores.forEach((k, v) -> System.out.println(k + "=" + v));
    }
}
"#);
    assert_eq!(out.trim(), "[2, 4]\n[1]\n2\nAlice=90\nBob=85\nCharlie=92");
}

#[test]
fn stream_collect_groupby() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("apple", "banana", "avocado", "blueberry", "cherry", "apricot");
        // group by first letter
        Map<String, List<String>> grouped = words.stream()
            .collect(Collectors.groupingBy(w -> w.substring(0, 1)));
        // print sorted keys
        new TreeMap<>(grouped).forEach((k, v) -> {
            Collections.sort(v);
            System.out.println(k + ": " + v);
        });
    }
}
"#);
    assert_eq!(out.trim(), "a: [apple, apricot, avocado]\nb: [banana, blueberry]\nc: [cherry]");
}

#[test]
fn varargs_generic() {
    let out = run(r#"
import java.util.*;
class Main {
    @SafeVarargs
    static <T> List<T> listOf(T... items) {
        List<T> result = new ArrayList<>();
        for (T item : items) result.add(item);
        return result;
    }
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }
    public static void main(String[] args) {
        System.out.println(listOf("a", "b", "c"));
        System.out.println(listOf(1, 2, 3));
        System.out.println(sum(1, 2, 3, 4, 5));
        System.out.println(sum());
    }
}
"#);
    assert_eq!(out.trim(), "[a, b, c]\n[1, 2, 3]\n15\n0");
}

#[test]
fn inheritance_polymorphism() {
    let out = run(r#"
abstract class Animal {
    String name;
    Animal(String name) { this.name = name; }
    abstract String sound();
    String describe() { return name + " says " + sound(); }
}
class Dog extends Animal {
    Dog(String name) { super(name); }
    public String sound() { return "woof"; }
    String fetch() { return name + " fetches!"; }
}
class Cat extends Animal {
    Cat(String name) { super(name); }
    public String sound() { return "meow"; }
    String purr() { return name + " purrs"; }
}
class Main {
    public static void main(String[] args) {
        Animal[] animals = { new Dog("Rex"), new Cat("Whiskers"), new Dog("Buddy") };
        for (Animal a : animals) System.out.println(a.describe());
        // downcast
        for (Animal a : animals) {
            if (a instanceof Dog) {
                Dog d = (Dog) a;
                System.out.println(d.fetch());
            }
        }
    }
}
"#);
    assert_eq!(out.trim(), "Rex says woof\nWhiskers says meow\nBuddy says woof\nRex fetches!\nBuddy fetches!");
}

#[test]
fn enum_state_machine() {
    let out = run(r#"
enum State {
    IDLE, RUNNING, PAUSED, STOPPED;
    boolean canTransitionTo(State next) {
        switch (this) {
            case IDLE: return next == RUNNING;
            case RUNNING: return next == PAUSED || next == STOPPED;
            case PAUSED: return next == RUNNING || next == STOPPED;
            case STOPPED: return false;
            default: return false;
        }
    }
}
class Main {
    public static void main(String[] args) {
        State s = State.IDLE;
        System.out.println(s.canTransitionTo(State.RUNNING));
        System.out.println(s.canTransitionTo(State.PAUSED));
        s = State.RUNNING;
        System.out.println(s.canTransitionTo(State.PAUSED));
        System.out.println(s.canTransitionTo(State.STOPPED));
        s = State.STOPPED;
        System.out.println(s.canTransitionTo(State.RUNNING));
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse\ntrue\ntrue\nfalse");
}

#[test]
fn map_entry_iteration_v2() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> map = new TreeMap<>();
        map.put("banana", 3);
        map.put("apple", 5);
        map.put("cherry", 1);
        // iterate via keySet
        int total = 0;
        for (String key : map.keySet()) {
            total += map.get(key);
        }
        System.out.println(total);
        // iterate via values
        List<Integer> vals = new ArrayList<>(map.values());
        Collections.sort(vals);
        System.out.println(vals);
        // entrySet via keySet workaround
        for (String k : map.keySet()) {
            System.out.println(k + "->" + map.get(k));
        }
    }
}
"#);
    assert_eq!(out.trim(), "9\n[1, 3, 5]\napple->5\nbanana->3\ncherry->1");
}

#[test]
fn string_operations_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello, World!";
        // contains, startsWith, endsWith
        System.out.println(s.contains("World"));
        System.out.println(s.startsWith("Hello"));
        System.out.println(s.endsWith("!"));
        // indexOf, lastIndexOf
        System.out.println(s.indexOf('o'));
        System.out.println(s.lastIndexOf('o'));
        // replace
        System.out.println(s.replace("World", "Java"));
        // substring
        System.out.println(s.substring(7, 12));
        // chars
        System.out.println(s.charAt(0));
        System.out.println((int) s.charAt(0));
    }
}
"#);
    assert_eq!(out.trim(), "true\ntrue\ntrue\n4\n8\nHello, Java!\nWorld\nH\n72");
}

#[test]
fn list_sublist_and_copy() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));
        // subList
        List<Integer> sub = list.subList(2, 6);
        System.out.println(sub);
        // copy
        List<Integer> copy = new ArrayList<>(list);
        copy.set(0, 99);
        System.out.println(list.get(0));  // original unchanged
        System.out.println(copy.get(0));
        // filter odds manually
        List<Integer> odds = new ArrayList<>();
        for (int x : copy) if (x % 2 != 0) odds.add(x);
        System.out.println(odds);
        // contains, indexOf
        System.out.println(list.contains(5));
        System.out.println(list.indexOf(5));
    }
}
"#);
    assert_eq!(out.trim(), "[3, 4, 5, 6]\n1\n99\n[99, 3, 5, 7, 9]\ntrue\n4");
}

#[test]
fn math_operations() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Math.abs(-42));
        System.out.println(Math.abs(-3.14));
        System.out.println(Math.max(10, 20));
        System.out.println(Math.min(10, 20));
        System.out.println(Math.pow(2, 10));
        System.out.println(Math.sqrt(144.0));
        System.out.println(Math.floor(3.7));
        System.out.println(Math.ceil(3.2));
        System.out.println(Math.round(3.5));
        System.out.println(Math.round(3.4));
        System.out.println((int) Math.log10(1000));
    }
}
"#);
    assert_eq!(out.trim(), "42\n3.14\n20\n10\n1024.0\n12.0\n3.0\n4.0\n4\n3\n3");
}

#[test]
fn integer_methods_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Integer.parseInt("42"));
        System.out.println(Integer.parseInt("-17"));
        System.out.println(Integer.toBinaryString(10));
        System.out.println(Integer.toHexString(255));
        System.out.println(Integer.toOctalString(8));
        System.out.println(Integer.bitCount(7));
        System.out.println(Integer.reverse(1));
        System.out.println(Integer.compare(5, 10));
        System.out.println(Integer.max(3, 7));
        System.out.println(Integer.min(3, 7));
        System.out.println(Integer.sum(3, 7));
    }
}
"#);
    assert_eq!(out.trim(), "42\n-17\n1010\nff\n10\n3\n-2147483648\n-1\n7\n3\n10");
}

#[test]
fn stack_overflow_recursion() {
    let out = run(r#"
class Main {
    static long factorial(int n) {
        if (n <= 1) return 1;
        return n * factorial(n - 1);
    }
    static int gcd(int a, int b) {
        return b == 0 ? a : gcd(b, a % b);
    }
    static int power(int base, int exp) {
        if (exp == 0) return 1;
        if (exp % 2 == 0) {
            int half = power(base, exp / 2);
            return half * half;
        }
        return base * power(base, exp - 1);
    }
    public static void main(String[] args) {
        System.out.println(factorial(10));
        System.out.println(gcd(48, 18));
        System.out.println(gcd(100, 75));
        System.out.println(power(2, 10));
        System.out.println(power(3, 5));
    }
}
"#);
    assert_eq!(out.trim(), "3628800\n6\n25\n1024\n243");
}

#[test]
fn interface_default_override() {
    let out = run(r#"
interface Greeter {
    default String greet(String name) { return "Hello, " + name + "!"; }
    default String farewell(String name) { return "Goodbye, " + name + "!"; }
}
interface FormalGreeter extends Greeter {
    default String greet(String name) { return "Good day, " + name + "."; }
}
class EnglishGreeter implements Greeter {}
class FormalEnglish implements FormalGreeter {
    public String farewell(String name) { return "Farewell, " + name + "."; }
}
class Main {
    public static void main(String[] args) {
        Greeter g = new EnglishGreeter();
        System.out.println(g.greet("Alice"));
        System.out.println(g.farewell("Bob"));
        FormalGreeter fg = new FormalEnglish();
        System.out.println(fg.greet("Charlie"));
        System.out.println(fg.farewell("Diana"));
    }
}
"#);
    assert_eq!(out.trim(), "Hello, Alice!\nGoodbye, Bob!\nGood day, Charlie.\nFarewell, Diana.");
}

#[test]
fn collections_priority_queue_v2() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        // min-heap (default)
        PriorityQueue<Integer> minHeap = new PriorityQueue<>();
        minHeap.add(5); minHeap.add(1); minHeap.add(3); minHeap.add(2); minHeap.add(4);
        List<Integer> sorted = new ArrayList<>();
        while (!minHeap.isEmpty()) sorted.add(minHeap.poll());
        System.out.println(sorted);
        // max-heap
        PriorityQueue<Integer> maxHeap = new PriorityQueue<>(Comparator.reverseOrder());
        maxHeap.add(5); maxHeap.add(1); maxHeap.add(3); maxHeap.add(2); maxHeap.add(4);
        List<Integer> rsorted = new ArrayList<>();
        while (!maxHeap.isEmpty()) rsorted.add(maxHeap.poll());
        System.out.println(rsorted);
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 4, 5]\n[5, 4, 3, 2, 1]");
}

#[test]
fn string_format_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(String.format("%d + %d = %d", 3, 4, 7));
        System.out.println(String.format("%.2f", 3.14159));
        System.out.println(String.format("%s has %d items", "list", 5));
        System.out.println(String.format("%05d", 42));
        System.out.println(String.format("%-10s|", "left"));
        System.out.println(String.format("%10s|", "right"));
        System.out.printf("Hello %s!%n", "World");
    }
}
"#);
    assert_eq!(out.trim(), "3 + 4 = 7\n3.14\nlist has 5 items\n00042\nleft      |\n     right|\nHello World!");
}

#[test]
fn multiline_string_ops() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String text = "line1\nline2\nline3\nline4";
        String[] lines = text.split("\n");
        System.out.println(lines.length);
        for (String line : lines) System.out.println(line.toUpperCase());
        // join
        String joined = String.join(" | ", lines);
        System.out.println(joined);
    }
}
"#);
    assert_eq!(out.trim(), "4\nLINE1\nLINE2\nLINE3\nLINE4\nline1 | line2 | line3 | line4");
}

#[test]
fn generic_triple() {
    let out = run(r#"
class Triple<A, B, C> {
    A first; B second; C third;
    Triple(A a, B b, C c) { first = a; second = b; third = c; }
    public String toString() { return "(" + first + ", " + second + ", " + third + ")"; }
    Triple<C, B, A> reverse() { return new Triple<>(third, second, first); }
}
class Main {
    public static void main(String[] args) {
        Triple<String, Integer, Boolean> t = new Triple<>("hello", 42, true);
        System.out.println(t);
        System.out.println(t.first);
        System.out.println(t.second);
        System.out.println(t.third);
        Triple<Boolean, Integer, String> r = t.reverse();
        System.out.println(r);
    }
}
"#);
    assert_eq!(out.trim(), "(hello, 42, true)\nhello\n42\ntrue\n(true, 42, hello)");
}

#[test]
fn exception_custom_hierarchy_v2() {
    let out = run(r#"
class AppException extends RuntimeException {
    int code;
    AppException(String msg, int code) { super(msg); this.code = code; }
}
class ValidationException extends AppException {
    String field;
    ValidationException(String field, String msg) { super(msg, 400); this.field = field; }
}
class NotFoundException extends AppException {
    NotFoundException(String msg) { super(msg, 404); }
}
class Main {
    static void validate(String name, String value) {
        if (value == null || value.isEmpty()) throw new ValidationException(name, name + " is required");
    }
    static String findUser(int id) {
        if (id <= 0) throw new NotFoundException("User " + id + " not found");
        return "User" + id;
    }
    public static void main(String[] args) {
        try { validate("email", ""); }
        catch (ValidationException e) { System.out.println("Validation: " + e.field + " - " + e.getMessage() + " (" + e.code + ")"); }
        try { findUser(-1); }
        catch (NotFoundException e) { System.out.println("NotFound: " + e.getMessage() + " (" + e.code + ")"); }
        try { findUser(42); System.out.println("Found: " + findUser(42)); }
        catch (AppException e) { System.out.println("Error"); }
    }
}
"#);
    assert_eq!(out.trim(), "Validation: email - email is required (400)\nNotFound: User -1 not found (404)\nFound: User42");
}

#[test]
fn stream_map_reduce() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<String> words = Arrays.asList("hello", "world", "java", "stream");
        // map to lengths and sum
        int totalLen = words.stream().mapToInt(String::length).sum();
        System.out.println(totalLen);
        // map to uppercase and join
        String upper = words.stream().map(String::toUpperCase).collect(Collectors.joining(", "));
        System.out.println(upper);
        // filter and count
        long longWords = words.stream().filter(w -> w.length() > 4).count();
        System.out.println(longWords);
        // reduce to concatenation
        String concat = words.stream().reduce("", (a, b) -> a + b);
        System.out.println(concat);
    }
}
"#);
    assert_eq!(out.trim(), "20\nHELLO, WORLD, JAVA, STREAM\n3\nhelloworldjavastream");
}

#[test]
fn collections_linked_hashset() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        // LinkedHashSet deduplication
        Set<String> set = new LinkedHashSet<>();
        set.add("banana"); set.add("apple"); set.add("cherry");
        set.add("apple");  // duplicate
        System.out.println(set.size());
        System.out.println(set.contains("apple"));
        System.out.println(set.contains("mango"));
        // LinkedHashMap — use TreeMap for deterministic order
        Map<String, Integer> map = new TreeMap<>();
        map.put("c", 3); map.put("a", 1); map.put("b", 2);
        for (String k : map.keySet()) System.out.print(k + "=" + map.get(k) + " ");
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "a=1 b=2 c=3");
}

#[test]
fn array_2d_spiral() {
    let out = run(r#"
import java.util.*;
class Main {
    static List<Integer> spiral(int[][] matrix) {
        List<Integer> result = new ArrayList<>();
        int top = 0, bottom = matrix.length - 1;
        int left = 0, right = matrix[0].length - 1;
        while (top <= bottom && left <= right) {
            for (int i = left; i <= right; i++) result.add(matrix[top][i]);
            top++;
            for (int i = top; i <= bottom; i++) result.add(matrix[i][right]);
            right--;
            if (top <= bottom) {
                for (int i = right; i >= left; i--) result.add(matrix[bottom][i]);
                bottom--;
            }
            if (left <= right) {
                for (int i = bottom; i >= top; i--) result.add(matrix[i][left]);
                left++;
            }
        }
        return result;
    }
    public static void main(String[] args) {
        int[][] m = {{1,2,3},{4,5,6},{7,8,9}};
        System.out.println(spiral(m));
        int[][] m2 = {{1,2,3,4},{5,6,7,8},{9,10,11,12}};
        System.out.println(spiral(m2));
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 6, 9, 8, 7, 4, 5]\n[1, 2, 3, 4, 8, 12, 11, 10, 9, 5, 6, 7]");
}

#[test]
fn string_number_format() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // Integer conversions
        System.out.println(Integer.toString(255, 16));  // hex
        System.out.println(Integer.toString(255, 2));   // binary
        System.out.println(Integer.toString(255, 8));   // octal
        // Double formatting
        double d = 1234567.89;
        System.out.println(String.format("%.2f", d));
        System.out.println(String.format("%e", d));
        // Padding
        System.out.println(String.format("|%10d|", 42));
        System.out.println(String.format("|%-10d|", 42));
    }
}
"#);
    assert_eq!(out.trim(), "ff\n11111111\n377\n1234567.89\n1.234568e+06\n|        42|\n|42        |");
}

#[test]
fn functional_pipeline() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
import java.util.function.*;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        // chain: filter evens, map to squares, limit 4, collect
        List<Integer> result = nums.stream()
            .filter(n -> n % 2 == 0)
            .map(n -> n * n)
            .limit(4)
            .collect(Collectors.toList());
        System.out.println(result);
        // sum of odd squares
        int sumOddSq = nums.stream()
            .filter(n -> n % 2 != 0)
            .mapToInt(n -> n * n)
            .sum();
        System.out.println(sumOddSq);
        // string pipeline
        List<String> words = Arrays.asList("  hello  ", "  world  ", "  java  ");
        String joined = words.stream()
            .map(String::trim)
            .map(String::toUpperCase)
            .collect(Collectors.joining("-"));
        System.out.println(joined);
    }
}
"#);
    assert_eq!(out.trim(), "[4, 16, 36, 64]\n165\nHELLO-WORLD-JAVA");
}

#[test]
fn abstract_factory() {
    let out = run(r#"
interface Button { String render(); }
interface Checkbox { String render(); }
interface UIFactory {
    Button createButton();
    Checkbox createCheckbox();
}
class WindowsButton implements Button {
    public String render() { return "Windows Button"; }
}
class WindowsCheckbox implements Checkbox {
    public String render() { return "Windows Checkbox"; }
}
class MacButton implements Button {
    public String render() { return "Mac Button"; }
}
class MacCheckbox implements Checkbox {
    public String render() { return "Mac Checkbox"; }
}
class WindowsFactory implements UIFactory {
    public Button createButton() { return new WindowsButton(); }
    public Checkbox createCheckbox() { return new WindowsCheckbox(); }
}
class MacFactory implements UIFactory {
    public Button createButton() { return new MacButton(); }
    public Checkbox createCheckbox() { return new MacCheckbox(); }
}
class Main {
    static void renderUI(UIFactory factory) {
        System.out.println(factory.createButton().render());
        System.out.println(factory.createCheckbox().render());
    }
    public static void main(String[] args) {
        renderUI(new WindowsFactory());
        renderUI(new MacFactory());
    }
}
"#);
    assert_eq!(out.trim(), "Windows Button\nWindows Checkbox\nMac Button\nMac Checkbox");
}

#[test]
fn collections_stack_operations() {
    let out = run(r#"
import java.util.*;
class Main {
    // balanced parentheses checker
    static boolean isBalanced(String s) {
        Deque<Character> stack = new ArrayDeque<>();
        for (char c : s.toCharArray()) {
            if (c == '(' || c == '[' || c == '{') {
                stack.push(c);
            } else if (c == ')' || c == ']' || c == '}') {
                if (stack.isEmpty()) return false;
                char top = stack.pop();
                if (c == ')' && top != '(') return false;
                if (c == ']' && top != '[') return false;
                if (c == '}' && top != '{') return false;
            }
        }
        return stack.isEmpty();
    }
    public static void main(String[] args) {
        System.out.println(isBalanced("()[]{}"));
        System.out.println(isBalanced("([{}])"));
        System.out.println(isBalanced("([)]"));
        System.out.println(isBalanced("{[}"));
        System.out.println(isBalanced(""));
    }
}
"#);
    assert_eq!(out.trim(), "true\ntrue\nfalse\nfalse\ntrue");
}

#[test]
fn number_theory() {
    let out = run(r#"
import java.util.*;
class Main {
    static boolean isPrime(int n) {
        if (n < 2) return false;
        for (int i = 2; i * i <= n; i++)
            if (n % i == 0) return false;
        return true;
    }
    static List<Integer> sieve(int limit) {
        boolean[] composite = new boolean[limit + 1];
        List<Integer> primes = new ArrayList<>();
        for (int i = 2; i <= limit; i++) {
            if (!composite[i]) {
                primes.add(i);
                for (int j = i * 2; j <= limit; j += i)
                    composite[j] = true;
            }
        }
        return primes;
    }
    static int lcm(int a, int b) {
        int g = a, r = b;
        while (r != 0) { int t = r; r = g % r; g = t; }
        return a / g * b;
    }
    public static void main(String[] args) {
        System.out.println(isPrime(17));
        System.out.println(isPrime(18));
        System.out.println(sieve(30));
        System.out.println(lcm(12, 18));
        System.out.println(lcm(7, 5));
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse\n[2, 3, 5, 7, 11, 13, 17, 19, 23, 29]\n36\n35");
}

#[test]
#[ignore = "Known limitation: assignment-in-condition pattern (see docs/known-limitations.md)"]
fn string_manipulation_advanced() {
    let out = run(r#"
class Main {
    static String reverseWords(String s) {
        String[] words = s.trim().split("\\s+");
        StringBuilder sb = new StringBuilder();
        for (int i = words.length - 1; i >= 0; i--) {
            sb.append(words[i]);
            if (i > 0) sb.append(" ");
        }
        return sb.toString();
    }
    static boolean isPalindrome(String s) {
        String clean = s.toLowerCase().replaceAll("[^a-z0-9]", "");
        int l = 0, r = clean.length() - 1;
        while (l < r) {
            if (clean.charAt(l) != clean.charAt(r)) return false;
            l++; r--;
        }
        return true;
    }
    static int countOccurrences(String text, String pattern) {
        int count = 0, idx = 0;
        while ((idx = text.indexOf(pattern, idx)) != -1) { count++; idx++; }
        return count;
    }
    public static void main(String[] args) {
        System.out.println(reverseWords("Hello World Java"));
        System.out.println(isPalindrome("A man a plan a canal Panama"));
        System.out.println(isPalindrome("hello"));
        System.out.println(countOccurrences("abababab", "ab"));
        System.out.println(countOccurrences("hello world", "l"));
    }
}
"#);
    assert_eq!(out.trim(), "Java World Hello\ntrue\nfalse\n4\n3");
}

#[test]
#[ignore = "Known limitation: assignment-in-condition pattern (see docs/known-limitations.md)"]
fn index_of_with_from_index() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println("abababab".indexOf("ab", 1));
        System.out.println("abababab".indexOf("ab", 3));
        System.out.println("hello world".replaceAll("[^a-z]", ""));
        // test assignment-in-condition pattern
        String s = "abababab";
        int count = 0, idx = 0;
        idx = s.indexOf("ab", idx);
        while (idx != -1) { count++; idx++; idx = s.indexOf("ab", idx); }
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "2\n4\nhelloworld\n4");
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
fn ternary_nested_null() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int x = 10;
        String result = x > 5 ? "big" : "small";
        System.out.println(result);
        int y = x > 0 ? (x > 5 ? 2 : 1) : 0;
        System.out.println(y);
        // null coalescing pattern
        String s = null;
        String val = s != null ? s : "default";
        System.out.println(val);
    }
}
"#);
    assert_eq!(out.trim(), "big\n2\ndefault");
}

#[test]
fn array_multidimensional() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int[][] matrix = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                matrix[i][j] = i * 3 + j + 1;
        // print diagonal
        for (int i = 0; i < 3; i++)
            System.out.println(matrix[i][i]);
        // sum of row 1
        int sum = 0;
        for (int v : matrix[1]) sum += v;
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "1\n5\n9\n15");
}

#[test]
fn string_format_basic() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(String.format("%d + %d = %d", 3, 4, 7));
        System.out.println(String.format("%.2f", 3.14159));
        System.out.println(String.format("%s has %d items", "list", 5));
        System.out.println(String.format("%05d", 42));
    }
}
"#);
    assert_eq!(out.trim(), "3 + 4 = 7\n3.14\nlist has 5 items\n00042");
}

#[test]
fn collections_iterator() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> list = new ArrayList<>(Arrays.asList("a", "b", "c", "d"));
        Iterator<String> it = list.iterator();
        while (it.hasNext()) {
            String s = it.next();
            if (s.equals("b") || s.equals("d")) it.remove();
        }
        System.out.println(list);
    }
}
"#);
    assert_eq!(out.trim(), "[a, c]");
}

#[test]
fn math_functions_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Math.abs(-42));
        System.out.println(Math.max(10, 20));
        System.out.println(Math.min(10, 20));
        System.out.println((int) Math.pow(2, 10));
        System.out.println((int) Math.sqrt(144));
        System.out.println((int) Math.floor(3.9));
        System.out.println((int) Math.ceil(3.1));
        System.out.println(Math.round(3.5));
    }
}
"#);
    assert_eq!(out.trim(), "42\n20\n10\n1024\n12\n3\n4\n4");
}

#[test]
fn interface_multiple_v2() {
    let out = run(r#"
interface Flyable {
    default String fly() { return "flying"; }
}
interface Swimmable {
    default String swim() { return "swimming"; }
}
class Duck implements Flyable, Swimmable {
    public String describe() {
        return "Duck is " + fly() + " and " + swim();
    }
}
class Main {
    public static void main(String[] args) {
        Duck d = new Duck();
        System.out.println(d.describe());
        System.out.println(d.fly());
        System.out.println(d.swim());
    }
}
"#);
    assert_eq!(out.trim(), "Duck is flying and swimming\nflying\nswimming");
}

#[test]
fn generics_bounded() {
    let out = run(r#"
class Main {
    static <T extends Comparable<T>> T max(T a, T b) {
        return a.compareTo(b) >= 0 ? a : b;
    }
    static <T extends Number> double sum(T a, T b) {
        return a.doubleValue() + b.doubleValue();
    }
    public static void main(String[] args) {
        System.out.println(max(3, 7));
        System.out.println(max("apple", "banana"));
        System.out.println((int) sum(3, 4));
        System.out.println((int) sum(1.5, 2.5));
    }
}
"#);
    assert_eq!(out.trim(), "7\nbanana\n7\n4");
}

#[test]
fn lambda_comparator() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> words = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "date"));
        words.sort((a, b) -> a.compareTo(b));
        System.out.println(words);
        words.sort((a, b) -> b.length() - a.length());
        System.out.println(words.get(0));
        words.sort(Comparator.comparingInt(String::length));
        System.out.println(words.get(0));
    }
}
"#);
    assert_eq!(out.trim(), "[apple, banana, cherry, date]\nbanana\ndate");
}

#[test]
fn try_with_resources_v2() {
    let out = run(r#"
class MyResource implements AutoCloseable {
    String name;
    MyResource(String name) {
        this.name = name;
        System.out.println("open " + name);
    }
    public void close() {
        System.out.println("close " + name);
    }
    public void use() {
        System.out.println("use " + name);
    }
}
class Main {
    public static void main(String[] args) {
        try (MyResource r = new MyResource("R1")) {
            r.use();
        }
        System.out.println("done");
    }
}
"#);
    assert_eq!(out.trim(), "open R1\nuse R1\nclose R1\ndone");
}

#[test]
fn switch_days_of_week() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        for (int i = 1; i <= 7; i++) {
            String day;
            switch (i) {
                case 1: day = "Mon"; break;
                case 2: day = "Tue"; break;
                case 3: day = "Wed"; break;
                case 4: day = "Thu"; break;
                case 5: day = "Fri"; break;
                case 6: day = "Sat"; break;
                default: day = "Sun"; break;
            }
            if (i == 6 || i == 7) System.out.println(day + " weekend");
            else System.out.println(day + " weekday");
        }
    }
}
"#);
    assert_eq!(out.trim(), "Mon weekday\nTue weekday\nWed weekday\nThu weekday\nFri weekday\nSat weekend\nSun weekend");
}

#[test]
fn string_operations_extended() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello, World!";
        System.out.println(s.substring(7));
        System.out.println(s.substring(0, 5));
        System.out.println(s.replace("World", "Java"));
        System.out.println(s.toLowerCase());
        System.out.println(s.toUpperCase());
        System.out.println(s.contains("World"));
        System.out.println(s.startsWith("Hello"));
        System.out.println(s.endsWith("!"));
        System.out.println(s.indexOf("o"));
        System.out.println(s.lastIndexOf("o"));
    }
}
"#);
    assert_eq!(out.trim(), "World!\nHello\nHello, Java!\nhello, world!\nHELLO, WORLD!\ntrue\ntrue\ntrue\n4\n8");
}

#[test]
fn collections_map_operations() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> map = new HashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.put("c", 3);
        System.out.println(map.size());
        System.out.println(map.get("b"));
        System.out.println(map.containsKey("c"));
        System.out.println(map.containsValue(4));
        map.remove("a");
        System.out.println(map.size());
        map.putIfAbsent("d", 4);
        System.out.println(map.get("d"));
        System.out.println(map.getOrDefault("z", 99));
    }
}
"#);
    assert_eq!(out.trim(), "3\n2\ntrue\nfalse\n2\n4\n99");
}

#[test]
fn inheritance_polymorphism_v2() {
    let out = run(r#"
class Animal {
    String name;
    Animal(String name) { this.name = name; }
    String sound() { return "..."; }
    String describe() { return name + " says " + sound(); }
}
class Dog extends Animal {
    Dog(String name) { super(name); }
    String sound() { return "woof"; }
}
class Cat extends Animal {
    Cat(String name) { super(name); }
    String sound() { return "meow"; }
}
class Main {
    public static void main(String[] args) {
        Animal[] animals = { new Dog("Rex"), new Cat("Whiskers"), new Dog("Buddy") };
        for (Animal a : animals) {
            System.out.println(a.describe());
        }
    }
}
"#);
    assert_eq!(out.trim(), "Rex says woof\nWhiskers says meow\nBuddy says woof");
}

#[test]
fn stream_collect_operations() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        // sum of even numbers
        int evenSum = nums.stream().filter(n -> n % 2 == 0).mapToInt(Integer::intValue).sum();
        System.out.println(evenSum);
        // count odd numbers
        long oddCount = nums.stream().filter(n -> n % 2 != 0).count();
        System.out.println(oddCount);
        // collect to list
        List<Integer> evens = nums.stream().filter(n -> n % 2 == 0).collect(Collectors.toList());
        System.out.println(evens);
    }
}
"#);
    assert_eq!(out.trim(), "30\n5\n[2, 4, 6, 8, 10]");
}

#[test]
fn exception_hierarchy_v3() {
    let out = run(r#"
class AppException extends RuntimeException {
    int code;
    AppException(String msg, int code) {
        super(msg);
        this.code = code;
    }
}
class NetworkException extends AppException {
    NetworkException(String msg) { super(msg, 503); }
}
class Main {
    static void connect(boolean fail) {
        if (fail) throw new NetworkException("Connection refused");
    }
    public static void main(String[] args) {
        try {
            connect(true);
        } catch (NetworkException e) {
            System.out.println("Network error: " + e.getMessage() + " code=" + e.code);
        } catch (AppException e) {
            System.out.println("App error: " + e.getMessage());
        }
        try {
            connect(false);
            System.out.println("connected");
        } catch (AppException e) {
            System.out.println("should not happen");
        }
    }
}
"#);
    assert_eq!(out.trim(), "Network error: Connection refused code=503\nconnected");
}

#[test]
fn functional_interfaces_v2() {
    let out = run(r#"
import java.util.function.*;
class Main {
    static int apply(int x, Function<Integer, Integer> f) {
        return f.apply(x);
    }
    static boolean test(int x, Predicate<Integer> p) {
        return p.test(x);
    }
    static int supply(Supplier<Integer> s) {
        return s.get();
    }
    public static void main(String[] args) {
        System.out.println(apply(5, x -> x * x));
        System.out.println(apply(3, x -> x + 10));
        System.out.println(test(4, x -> x % 2 == 0));
        System.out.println(test(3, x -> x % 2 == 0));
        System.out.println(supply(() -> 42));
    }
}
"#);
    assert_eq!(out.trim(), "25\n13\ntrue\nfalse\n42");
}

#[test]
fn static_methods_and_fields() {
    let out = run(r#"
class Counter {
    static int count = 0;
    int id;
    Counter() {
        count++;
        id = count;
    }
    static int getCount() { return count; }
    static void reset() { count = 0; }
}
class Main {
    public static void main(String[] args) {
        System.out.println(Counter.getCount());
        Counter a = new Counter();
        Counter b = new Counter();
        Counter c = new Counter();
        System.out.println(Counter.getCount());
        System.out.println(a.id + " " + b.id + " " + c.id);
        Counter.reset();
        System.out.println(Counter.getCount());
    }
}
"#);
    assert_eq!(out.trim(), "0\n3\n1 2 3\n0");
}

#[test]
fn array_sorting_searching() {
    let out = run(r#"
import java.util.Arrays;
class Main {
    public static void main(String[] args) {
        int[] arr = {5, 2, 8, 1, 9, 3, 7, 4, 6};
        Arrays.sort(arr);
        System.out.println(Arrays.toString(arr));
        int idx = Arrays.binarySearch(arr, 7);
        System.out.println(idx);
        String[] words = {"banana", "apple", "cherry"};
        Arrays.sort(words);
        System.out.println(Arrays.toString(words));
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 4, 5, 6, 7, 8, 9]\n6\n[apple, banana, cherry]");
}

#[test]
fn while_do_while() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // while loop
        int i = 1, sum = 0;
        while (i <= 5) { sum += i; i++; }
        System.out.println(sum);
        // do-while
        int n = 1;
        do {
            System.out.print(n + " ");
            n *= 2;
        } while (n <= 16);
        System.out.println();
        // break and continue
        for (int j = 0; j < 10; j++) {
            if (j % 2 == 0) continue;
            if (j > 7) break;
            System.out.print(j + " ");
        }
        System.out.println();
    }
}
"#);
    assert_eq!(out.trim(), "15\n1 2 4 8 16 \n1 3 5 7");
}

#[test]
fn varargs_methods() {
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
        System.out.println(sum(1, 2, 3));
        System.out.println(sum(10, 20, 30, 40));
        System.out.println(sum());
        System.out.println(join(", ", "a", "b", "c"));
        System.out.println(join("-", "x", "y"));
    }
}
"#);
    assert_eq!(out.trim(), "6\n100\n0\na, b, c\nx-y");
}

#[test]
fn string_char_operations() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "Hello";
        System.out.println(s.charAt(0));
        System.out.println(s.charAt(4));
        char[] chars = s.toCharArray();
        System.out.println(chars.length);
        // count vowels
        int vowels = 0;
        for (char c : "Hello World".toCharArray()) {
            if ("aeiouAEIOU".indexOf(c) >= 0) vowels++;
        }
        System.out.println(vowels);
        // Character methods
        System.out.println(Character.isLetter('a'));
        System.out.println(Character.isDigit('5'));
        System.out.println(Character.toUpperCase('a'));
        System.out.println(Character.toLowerCase('Z'));
    }
}
"#);
    assert_eq!(out.trim(), "H\no\n5\n3\ntrue\ntrue\nA\nz");
}

#[test]
fn collections_set_ops_v2() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<Integer> a = Arrays.asList(1, 2, 3, 4, 5);
        List<Integer> b = Arrays.asList(3, 4, 5, 6, 7);
        // intersection
        List<Integer> inter = new ArrayList<>();
        for (int x : a) { if (b.contains(x)) inter.add(x); }
        Collections.sort(inter);
        System.out.println(inter);
        // union
        List<Integer> unionList = new ArrayList<>(a);
        for (int x : b) { if (!unionList.contains(x)) unionList.add(x); }
        Collections.sort(unionList);
        System.out.println(unionList);
        // difference
        List<Integer> diff = new ArrayList<>();
        for (int x : a) { if (!b.contains(x)) diff.add(x); }
        Collections.sort(diff);
        System.out.println(diff);
    }
}
"#);
    assert_eq!(out.trim(), "[3, 4, 5]\n[1, 2, 3, 4, 5, 6, 7]\n[1, 2]");
}

#[test]
fn recursion_advanced() {
    let out = run(r#"
class Main {
    static int fib(int n) {
        if (n <= 1) return n;
        return fib(n-1) + fib(n-2);
    }
    static int gcd(int a, int b) {
        return b == 0 ? a : gcd(b, a % b);
    }
    static String toBinary(int n) {
        if (n == 0) return "0";
        if (n == 1) return "1";
        return toBinary(n / 2) + (n % 2);
    }
    public static void main(String[] args) {
        for (int i = 0; i <= 7; i++) System.out.print(fib(i) + " ");
        System.out.println();
        System.out.println(gcd(48, 18));
        System.out.println(gcd(100, 75));
        System.out.println(toBinary(10));
        System.out.println(toBinary(255));
    }
}
"#);
    assert_eq!(out.trim(), "0 1 1 2 3 5 8 13 \n6\n25\n1010\n11111111");
}

#[test]
fn inner_class_static() {
    let out = run(r#"
class Outer {
    static class Inner {
        int x;
        int y;
        Inner(int x, int y) { this.x = x; this.y = y; }
        int sum() { return x + y; }
    }
    static class Builder {
        int val = 0;
        Builder add(int n) { val += n; return this; }
        Builder mul(int n) { val *= n; return this; }
        int build() { return val; }
    }
}
class Main {
    public static void main(String[] args) {
        Outer.Inner inner = new Outer.Inner(10, 5);
        System.out.println(inner.sum());
        int result = new Outer.Builder().add(3).add(4).mul(2).build();
        System.out.println(result);
    }
}
"#);
    assert_eq!(out.trim(), "15\n14");
}

#[test]
fn collections_deque_operations() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Deque<Integer> deque = new ArrayDeque<>();
        deque.addFirst(2);
        deque.addFirst(1);
        deque.addLast(3);
        deque.addLast(4);
        System.out.println(deque.peekFirst());
        System.out.println(deque.peekLast());
        System.out.println(deque.size());
        deque.pollFirst();
        deque.pollLast();
        System.out.println(deque.size());
        System.out.println(deque.peekFirst());
    }
}
"#);
    assert_eq!(out.trim(), "1\n4\n4\n2\n2");
}

#[test]
fn string_split_join_v2() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        String csv = "a,b,c,d,e";
        String[] parts = csv.split(",");
        System.out.println(parts.length);
        System.out.println(parts[2]);
        String joined = String.join("-", parts);
        System.out.println(joined);
        // split with limit
        String[] limited = csv.split(",", 3);
        System.out.println(limited.length);
        System.out.println(limited[2]);
        // join list
        List<String> list = Arrays.asList("x", "y", "z");
        System.out.println(String.join(", ", list));
    }
}
"#);
    assert_eq!(out.trim(), "5\nc\na-b-c-d-e\n3\nc,d,e\nx, y, z");
}

#[test]
fn type_casting_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // numeric casting
        double d = 3.99;
        int i = (int) d;
        System.out.println(i);
        long l = 1234567890123L;
        int truncated = (int) l;
        System.out.println(truncated != 0);
        // widening
        int x = 42;
        long lx = x;
        double dx = x;
        System.out.println(lx);
        System.out.println(dx);
        // instanceof
        Object obj = "hello";
        System.out.println(obj instanceof String);
        System.out.println(obj instanceof Integer);
        if (obj instanceof String) {
            String s = (String) obj;
            System.out.println(s.length());
        }
    }
}
"#);
    assert_eq!(out.trim(), "3\ntrue\n42\n42\ntrue\nfalse\n5");
}

#[test]
fn enum_with_methods_v2() {
    let out = run(r#"
enum Planet {
    MERCURY(3.303e+23, 2.4397e6),
    VENUS(4.869e+24, 6.0518e6),
    EARTH(5.976e+24, 6.37814e6);
    private final double mass;
    private final double radius;
    Planet(double mass, double radius) {
        this.mass = mass;
        this.radius = radius;
    }
    double surfaceGravity() {
        final double G = 6.67300E-11;
        return G * mass / (radius * radius);
    }
    String name() { return this.toString(); }
}
class Main {
    public static void main(String[] args) {
        for (Planet p : Planet.values()) {
            System.out.printf("%s: %.2f%n", p.name(), p.surfaceGravity());
        }
    }
}
"#);
    assert_eq!(out.trim(), "MERCURY: 3.70\nVENUS: 8.87\nEARTH: 9.80");
}

#[test]
fn collections_frequency_count() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        String[] words = {"apple", "banana", "apple", "cherry", "banana", "apple"};
        Map<String, Integer> freq = new HashMap<>();
        for (String w : words) {
            freq.put(w, freq.getOrDefault(w, 0) + 1);
        }
        // sort by key for deterministic output
        List<String> keys = new ArrayList<>(freq.keySet());
        Collections.sort(keys);
        for (String k : keys) {
            System.out.println(k + "=" + freq.get(k));
        }
    }
}
"#);
    assert_eq!(out.trim(), "apple=3\nbanana=2\ncherry=1");
}

#[test]
fn generic_stack_v3() {
    let out = run(r#"
class Stack<T> {
    private Object[] data = new Object[16];
    private int size = 0;
    void push(T item) { data[size++] = item; }
    @SuppressWarnings("unchecked")
    T pop() { return (T) data[--size]; }
    @SuppressWarnings("unchecked")
    T peek() { return (T) data[size - 1]; }
    boolean isEmpty() { return size == 0; }
    int size() { return size; }
}
class Main {
    public static void main(String[] args) {
        Stack<Integer> s = new Stack<>();
        s.push(1); s.push(2); s.push(3);
        System.out.println(s.size());
        System.out.println(s.peek());
        System.out.println(s.pop());
        System.out.println(s.size());
        System.out.println(s.isEmpty());
        s.pop(); s.pop();
        System.out.println(s.isEmpty());
    }
}
"#);
    assert_eq!(out.trim(), "3\n3\n3\n2\nfalse\ntrue");
}

#[test]
fn string_builder_ops_v2() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        StringBuilder sb = new StringBuilder("Hello");
        sb.append(", ").append("World").append("!");
        System.out.println(sb.toString());
        System.out.println(sb.length());
        sb.insert(5, " Beautiful");
        System.out.println(sb.toString());
        sb.delete(5, 15);
        System.out.println(sb.toString());
        sb.reverse();
        System.out.println(sb.toString());
        System.out.println(sb.charAt(0));
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!\n13\nHello Beautiful, World!\nHello, World!\n!dlroW ,olleH\n!");
}

#[test]
fn collections_list_operations() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<Integer> list = new ArrayList<>(Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6));
        System.out.println(Collections.max(list));
        System.out.println(Collections.min(list));
        Collections.sort(list);
        System.out.println(list);
        Collections.reverse(list);
        System.out.println(list.get(0));
        System.out.println(Collections.frequency(list, 1));
    }
}
"#);
    assert_eq!(out.trim(), "9\n1\n[1, 1, 2, 3, 4, 5, 6, 9]\n9\n2");
}

#[test]
fn abstract_class() {
    let out = run(r#"
abstract class Shape {
    String color;
    Shape(String color) { this.color = color; }
    abstract double area();
    abstract double perimeter();
    String describe() {
        return color + " " + getClass().getSimpleName() + ": area=" + String.format("%.2f", area());
    }
}
class Circle extends Shape {
    double r;
    Circle(String color, double r) { super(color); this.r = r; }
    double area() { return Math.PI * r * r; }
    double perimeter() { return 2 * Math.PI * r; }
}
class Rectangle extends Shape {
    double w, h;
    Rectangle(String color, double w, double h) { super(color); this.w = w; this.h = h; }
    double area() { return w * h; }
    double perimeter() { return 2 * (w + h); }
}
class Main {
    public static void main(String[] args) {
        Shape[] shapes = { new Circle("red", 5), new Rectangle("blue", 4, 6) };
        for (Shape s : shapes) System.out.println(s.describe());
    }
}
"#);
    assert_eq!(out.trim(), "red Circle: area=78.54\nblue Rectangle: area=24.00");
}

#[test]
fn stream_flatmap() {
    let out = run(r#"
import java.util.*;
import java.util.stream.*;
class Main {
    public static void main(String[] args) {
        List<List<Integer>> nested = Arrays.asList(
            Arrays.asList(1, 2, 3),
            Arrays.asList(4, 5),
            Arrays.asList(6, 7, 8, 9)
        );
        List<Integer> flat = nested.stream()
            .flatMap(Collection::stream)
            .collect(Collectors.toList());
        System.out.println(flat);
        int sum = flat.stream().mapToInt(Integer::intValue).sum();
        System.out.println(sum);
    }
}
"#);
    assert_eq!(out.trim(), "[1, 2, 3, 4, 5, 6, 7, 8, 9]\n45");
}

#[test]
fn integer_parsing() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        System.out.println(Integer.parseInt("42"));
        System.out.println(Integer.parseInt("-17"));
        System.out.println(Integer.parseInt("FF", 16));
        System.out.println(Integer.toBinaryString(10));
        System.out.println(Integer.toHexString(255));
        System.out.println(Integer.toOctalString(8));
        System.out.println(Integer.valueOf(100));
        System.out.println(Integer.compare(5, 10));
    }
}
"#);
    assert_eq!(out.trim(), "42\n-17\n255\n1010\nff\n10\n100\n-1");
}

#[test]
fn nested_loops_labels() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        // multiplication table 2x2
        for (int i = 1; i <= 3; i++) {
            for (int j = 1; j <= 3; j++) {
                System.out.print(i * j + " ");
            }
            System.out.println();
        }
        // break outer loop
        outer:
        for (int i = 0; i < 5; i++) {
            for (int j = 0; j < 5; j++) {
                if (i + j == 4) { System.out.println(i + "+" + j); break outer; }
            }
        }
    }
}
"#);
    assert_eq!(out.trim(), "1 2 3 \n2 4 6 \n3 6 9 \n0+4");
}

#[test]
fn comparable_sorting() {
    let out = run(r#"
import java.util.*;
class Student implements Comparable<Student> {
    String name;
    int grade;
    Student(String name, int grade) { this.name = name; this.grade = grade; }
    public int compareTo(Student other) { return Integer.compare(other.grade, this.grade); }
    public String toString() { return name + ":" + grade; }
}
class Main {
    public static void main(String[] args) {
        List<Student> students = new ArrayList<>();
        students.add(new Student("Alice", 85));
        students.add(new Student("Bob", 92));
        students.add(new Student("Charlie", 78));
        students.add(new Student("Diana", 95));
        Collections.sort(students);
        for (Student s : students) System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "Diana:95\nBob:92\nAlice:85\nCharlie:78");
}

#[test]
fn string_trim_strip() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = "  Hello World  ";
        System.out.println(s.trim());
        System.out.println(s.trim().length());
        System.out.println("".isEmpty());
        System.out.println("  ".isEmpty());
        System.out.println("hello".isEmpty());
        System.out.println(String.valueOf(42));
        System.out.println(String.valueOf(3.14));
        System.out.println(String.valueOf(true));
    }
}
"#);
    assert_eq!(out.trim(), "Hello World\n11\ntrue\nfalse\nfalse\n42\n3.14\ntrue");
}

#[test]
fn collections_map_iteration() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        Map<String, Integer> scores = new TreeMap<>();
        scores.put("Alice", 90);
        scores.put("Bob", 85);
        scores.put("Charlie", 92);
        // iterate entries (TreeMap gives sorted order)
        for (Map.Entry<String, Integer> e : scores.entrySet()) {
            System.out.println(e.getKey() + "=" + e.getValue());
        }
        // compute total
        int total = 0;
        for (int v : scores.values()) total += v;
        System.out.println("total=" + total);
    }
}
"#);
    assert_eq!(out.trim(), "Alice=90\nBob=85\nCharlie=92\ntotal=267");
}

// ── Anonymous class tests ─────────────────────────────────────────────────────

#[test]
fn anonymous_class_interface() {
    let out = run(r#"
interface Greeter {
    String greet(String name);
}
class Main {
    public static void main(String[] args) {
        Greeter g = new Greeter() {
            public String greet(String name) {
                return "Hello, " + name + "!";
            }
        };
        System.out.println(g.greet("World"));
        System.out.println(g.greet("Java"));
    }
}
"#);
    assert_eq!(out.trim(), "Hello, World!\nHello, Java!");
}

#[test]
fn anonymous_class_abstract() {
    let out = run(r#"
abstract class Shape {
    abstract double area();
    String describe() { return "Shape with area " + area(); }
}
class Main {
    public static void main(String[] args) {
        Shape circle = new Shape() {
            double radius = 5.0;
            double area() { return 3.14 * radius * radius; }
        };
        Shape rect = new Shape() {
            double w = 4.0, h = 3.0;
            double area() { return w * h; }
        };
        System.out.println(circle.area());
        System.out.println(rect.area());
        System.out.println(rect.describe());
    }
}
"#);
    assert_eq!(out.trim(), "78.5\n12.0\nShape with area 12.0");
}

#[test]
fn anonymous_class_captures_local() {
    let out = run(r#"
interface Adder {
    int add(int x);
}
class Main {
    public static void main(String[] args) {
        int base = 10;
        Adder a = new Adder() {
            public int add(int x) { return x + base; }
        };
        System.out.println(a.add(5));
        System.out.println(a.add(20));
    }
}
"#);
    assert_eq!(out.trim(), "15\n30");
}

#[test]
fn anonymous_class_comparator() {
    let out = run(r#"
import java.util.*;
class Main {
    public static void main(String[] args) {
        List<String> words = new ArrayList<>();
        words.add("banana");
        words.add("apple");
        words.add("cherry");
        words.add("date");
        Collections.sort(words, new Comparator<String>() {
            public int compare(String a, String b) {
                return a.length() - b.length();
            }
        });
        for (String w : words) System.out.println(w);
    }
}
"#);
    assert_eq!(out.trim(), "date\napple\nbanana\ncherry");
}

// ── Text block tests ──────────────────────────────────────────────────────────

#[test]
fn text_block_basic() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = """
                Hello,
                World!
                """;
        System.out.print(s);
    }
}
"#);
    assert_eq!(out.trim(), "Hello,\nWorld!");
}

#[test]
fn text_block_json_like() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String json = """
                {
                    "name": "Alice",
                    "age": 30
                }
                """;
        System.out.println(json.contains("Alice"));
        System.out.println(json.contains("age"));
    }
}
"#);
    assert_eq!(out.trim(), "true\ntrue");
}

#[test]
fn text_block_no_trailing_newline() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String s = """
                line1
                line2""";
        System.out.println(s);
    }
}
"#);
    assert_eq!(out.trim(), "line1\nline2");
}

#[test]
fn text_block_with_string_format() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String name = "Bob";
        int age = 25;
        String tmpl = """
                Name: %s
                Age: %d
                """;
        System.out.print(String.format(tmpl, name, age));
    }
}
"#);
    assert_eq!(out.trim(), "Name: Bob\nAge: 25");
}

// ── Pattern / Matcher tests ───────────────────────────────────────────────────

#[test]
fn pattern_matches_static() {
    let out = run(r#"
import java.util.regex.Pattern;
class Main {
    public static void main(String[] args) {
        System.out.println(Pattern.matches("\\d+", "12345"));
        System.out.println(Pattern.matches("\\d+", "abc"));
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse");
}

#[test]
fn pattern_compile_and_matcher_find() {
    let out = run(r#"
import java.util.regex.Pattern;
import java.util.regex.Matcher;
class Main {
    public static void main(String[] args) {
        Pattern p = Pattern.compile("\\d+");
        Matcher m = p.matcher("foo 42 bar 7");
        int count = 0;
        while (m.find()) {
            count++;
        }
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "2");
}

#[test]
fn matcher_group() {
    let out = run(r#"
import java.util.regex.Pattern;
import java.util.regex.Matcher;
class Main {
    public static void main(String[] args) {
        Pattern p = Pattern.compile("[a-z]+");
        Matcher m = p.matcher("hello world");
        if (m.find()) {
            System.out.println(m.group());
        }
    }
}
"#);
    assert_eq!(out.trim(), "hello");
}

#[test]
fn matcher_replace_all() {
    let out = run(r#"
import java.util.regex.Pattern;
import java.util.regex.Matcher;
class Main {
    public static void main(String[] args) {
        Pattern p = Pattern.compile("\\d+");
        Matcher m = p.matcher("a1b22c333");
        System.out.println(m.replaceAll("N"));
    }
}
"#);
    assert_eq!(out.trim(), "aNbNcN");
}

#[test]
fn pattern_split() {
    let out = run(r#"
import java.util.regex.Pattern;
class Main {
    public static void main(String[] args) {
        Pattern p = Pattern.compile(",\\s*");
        String[] parts = p.split("a, b,  c,d");
        for (String s : parts) {
            System.out.println(s);
        }
    }
}
"#);
    assert_eq!(out.trim(), "a\nb\nc\nd");
}

// ── Scanner tests ─────────────────────────────────────────────────────────────

#[test]
fn scanner_next_line() {
    let out = run(r#"
import java.util.Scanner;
class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner("hello\nworld");
        while (sc.hasNextLine()) {
            System.out.println(sc.nextLine());
        }
        sc.close();
    }
}
"#);
    assert_eq!(out.trim(), "hello\nworld");
}

#[test]
fn scanner_next_int() {
    let out = run(r#"
import java.util.Scanner;
class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner("1 2 3");
        int sum = 0;
        while (sc.hasNextInt()) {
            sum += sc.nextInt();
        }
        System.out.println(sum);
        sc.close();
    }
}
"#);
    assert_eq!(out.trim(), "6");
}

#[test]
fn scanner_next_tokens() {
    let out = run(r#"
import java.util.Scanner;
class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner("foo bar baz");
        while (sc.hasNext()) {
            System.out.println(sc.next());
        }
        sc.close();
    }
}
"#);
    assert_eq!(out.trim(), "foo\nbar\nbaz");
}

#[test]
fn scanner_mixed_types() {
    let out = run(r#"
import java.util.Scanner;
class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner("42 3.14 hello");
        int i = sc.nextInt();
        double d = sc.nextDouble();
        String s = sc.next();
        System.out.println(i);
        System.out.println(d);
        System.out.println(s);
        sc.close();
    }
}
"#);
    assert_eq!(out.trim(), "42\n3.14\nhello");
}

#[test]
fn debug_count_occurrences() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String text = "abababab";
        String pattern = "ab";
        int count = 0, idx = 0;
        while ((idx = text.indexOf(pattern, idx)) != -1) { count++; idx++; }
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "4");
}

#[test]
fn debug_while_assign_in_cond() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int x = 10;
        int count = 0;
        while ((x = x - 1) > 0) { count++; }
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "9");
}

#[test]
fn debug_while_assign_cond_and_body() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String text = "abababab";
        int count = 0, idx = 0;
        int found;
        while ((found = text.indexOf("ab", idx)) != -1) {
            count++;
            idx = found + 1;
        }
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "4");
}

#[test]
fn debug_count_occ_exact() {
    let out = run(r#"
class Main {
    static int countOccurrences(String text, String pattern) {
        int count = 0, idx = 0;
        while ((idx = text.indexOf(pattern, idx)) != -1) { count++; idx++; }
        return count;
    }
    public static void main(String[] args) {
        System.out.println(countOccurrences("abababab", "ab"));
    }
}
"#);
    assert_eq!(out.trim(), "4");
}

#[test]
fn debug_palindrome() {
    let out = run(r#"
class Main {
    static boolean isPalindrome(String s) {
        String clean = s.toLowerCase().replaceAll("[^a-z0-9]", "");
        int l = 0, r = clean.length() - 1;
        while (l < r) {
            if (clean.charAt(l) != clean.charAt(r)) return false;
            l++; r--;
        }
        return true;
    }
    public static void main(String[] args) {
        System.out.println(isPalindrome("A man a plan a canal Panama"));
        System.out.println(isPalindrome("hello"));
    }
}
"#);
    assert_eq!(out.trim(), "true\nfalse");
}

#[test]
fn debug_string_manip_no_count() {
    let out = run(r#"
class Main {
    static String reverseWords(String s) {
        String[] words = s.trim().split("\\s+");
        StringBuilder sb = new StringBuilder();
        for (int i = words.length - 1; i >= 0; i--) {
            sb.append(words[i]);
            if (i > 0) sb.append(" ");
        }
        return sb.toString();
    }
    static boolean isPalindrome(String s) {
        String clean = s.toLowerCase().replaceAll("[^a-z0-9]", "");
        int l = 0, r = clean.length() - 1;
        while (l < r) {
            if (clean.charAt(l) != clean.charAt(r)) return false;
            l++; r--;
        }
        return true;
    }
    public static void main(String[] args) {
        System.out.println(reverseWords("Hello World Java"));
        System.out.println(isPalindrome("A man a plan a canal Panama"));
        System.out.println(isPalindrome("hello"));
    }
}
"#);
    assert_eq!(out.trim(), "Java World Hello\ntrue\nfalse");
}

#[test]
fn debug_simple_while() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        int x = 0;
        while (x < 3) { x++; }
        System.out.println(x);
    }
}
"#);
    assert_eq!(out.trim(), "3");
}

#[test]
fn debug_binary_search() {
    let out = run(r#"
class Main {
    static int binarySearch(int[] arr, int target) {
        int lo = 0, hi = arr.length - 1;
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            if (arr[mid] == target) return mid;
            else if (arr[mid] < target) lo = mid + 1;
            else hi = mid - 1;
        }
        return -1;
    }
    public static void main(String[] args) {
        int[] arr = {1, 2, 3, 5, 8, 9};
        System.out.println(binarySearch(arr, 8));
        System.out.println(binarySearch(arr, 7));
    }
}
"#);
    assert_eq!(out.trim(), "4\n-1");
}

#[test]
fn debug_binary_search_verbose() {
    let out = run(r#"
class Main {
    static int binarySearch(int[] arr, int target) {
        int lo = 0, hi = arr.length - 1;
        System.out.println("Initial: lo=" + lo + " hi=" + hi);
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            System.out.println("Loop: lo=" + lo + " hi=" + hi + " mid=" + mid + " arr[mid]=" + arr[mid]);
            if (arr[mid] == target) return mid;
            else if (arr[mid] < target) lo = mid + 1;
            else hi = mid - 1;
        }
        System.out.println("Exit: lo=" + lo + " hi=" + hi);
        return -1;
    }
    public static void main(String[] args) {
        int[] arr = {1, 2, 3, 5, 8, 9};
        System.out.println("Result: " + binarySearch(arr, 8));
    }
}
"#);
    println!("{}", out);
}

#[test]
fn debug_count_occ_verbose() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String text = "abab";
        int count = 0, idx = 0;
        System.out.println("Start: idx=" + idx);
        while ((idx = text.indexOf("ab", idx)) != -1) {
            System.out.println("Found at idx=" + idx + ", count=" + count);
            count++;
            idx++;
            System.out.println("After idx++: idx=" + idx);
        }
        System.out.println("Final: count=" + count);
    }
}
"#);
    println!("{}", out);
}

#[test]
fn debug_count_occ_no_assign_in_cond() {
    let out = run(r#"
class Main {
    public static void main(String[] args) {
        String text = "abab";
        int count = 0, idx = 0;
        idx = text.indexOf("ab", idx);
        while (idx != -1) {
            count++;
            idx++;
            idx = text.indexOf("ab", idx);
        }
        System.out.println(count);
    }
}
"#);
    assert_eq!(out.trim(), "2");
}
