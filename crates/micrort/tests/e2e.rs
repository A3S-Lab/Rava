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
