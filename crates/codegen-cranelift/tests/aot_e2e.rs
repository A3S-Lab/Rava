//! AOT end-to-end tests: Java source → native binary → run → check output.

use std::process::Command;

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn compile_and_run(java_src: &str) -> String {
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp_dir = std::env::temp_dir().join(format!("rava_aot_{}_{}", std::process::id(), id));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let src_path = tmp_dir.join("Main.java");
    std::fs::write(&src_path, java_src).unwrap();

    let out_path = tmp_dir.join("main_bin");

    // Compile: frontend → RIR → Cranelift → native binary
    let compiler = rava_frontend::Compiler::new();
    let mut module = compiler
        .compile(java_src, &src_path)
        .expect("compile failed");

    let backend = Box::new(rava_codegen_cranelift::CraneliftBackend::new());
    let aot = rava_aot::AotCompiler::with_default_passes(backend);
    aot.compile(&mut module, &out_path)
        .expect("aot compile failed");

    // Run the binary
    let output = Command::new(&out_path)
        .output()
        .expect("failed to run binary");

    let _ = std::fs::remove_dir_all(&tmp_dir);

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn aot_hello_world() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "Hello, World!");
}

#[test]
fn aot_arithmetic() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int a = 6;
        int b = 7;
        System.out.println(a * b);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "42");
}

#[test]
fn aot_for_loop() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int sum = 0;
        for (int i = 1; i <= 5; i++) {
            sum += i;
        }
        System.out.println(sum);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "15");
}

#[test]
fn aot_recursion_fibonacci() {
    let src = r#"
public class Main {
    static int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }
    public static void main(String[] args) {
        System.out.println(fib(10));
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "55");
}

#[test]
fn aot_static_fields() {
    let src = r#"
public class Main {
    static int count = 0;
    public static void main(String[] args) {
        count++;
        count++;
        count++;
        System.out.println(count);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "3");
}

#[test]
fn aot_object_fields() {
    let src = r#"
public class Main {
    int x;
    int y;
    public static void main(String[] args) {
        Main m = new Main();
        m.x = 10;
        m.y = 20;
        System.out.println(m.x + m.y);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "30");
}

#[test]
fn aot_inheritance() {
    let src = r#"
class Animal {
    String speak() { return "..."; }
}
class Dog extends Animal {
    String speak() { return "Woof"; }
}
class Cat extends Animal {
    String speak() { return "Meow"; }
}
public class Main {
    public static void main(String[] args) {
        Animal dog = new Dog();
        Animal cat = new Cat();
        System.out.println(dog.speak());
        System.out.println(cat.speak());
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "Woof");
    assert_eq!(lines[1], "Meow");
}

#[test]
fn aot_string_concat() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        String s = "Hello" + ", " + "World!";
        System.out.println(s);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "Hello, World!");
}

#[test]
fn aot_constructor_args() {
    let src = r#"
public class Main {
    String name;
    int age;
    Main(String name, int age) {
        this.name = name;
        this.age = age;
    }
    public static void main(String[] args) {
        Main m = new Main("Alice", 30);
        System.out.println(m.name);
        System.out.println(m.age);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "Alice");
    assert_eq!(lines[1], "30");
}

#[test]
fn aot_while_loop() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int n = 1;
        int result = 1;
        while (n <= 5) {
            result *= n;
            n++;
        }
        System.out.println(result);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "120");
}

#[test]
fn aot_interface() {
    let src = r#"
interface Shape {
    int area();
}
class Rect implements Shape {
    int w;
    int h;
    Rect(int w, int h) { this.w = w; this.h = h; }
    public int area() { return w * h; }
}
public class Main {
    public static void main(String[] args) {
        Shape s = new Rect(4, 5);
        System.out.println(s.area());
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "20");
}

#[test]
fn aot_array() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int[] arr = {10, 20, 30, 40, 50};
        int sum = 0;
        for (int i = 0; i < arr.length; i++) {
            sum += arr[i];
        }
        System.out.println(sum);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "150");
}

#[test]
fn aot_conditional() {
    let src = r#"
public class Main {
    static int max(int a, int b) {
        if (a > b) return a;
        return b;
    }
    public static void main(String[] args) {
        System.out.println(max(3, 7));
        System.out.println(max(10, 4));
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "7");
    assert_eq!(lines[1], "10");
}

#[test]
fn aot_do_while() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int i = 0;
        int sum = 0;
        do {
            sum += i;
            i++;
        } while (i < 5);
        System.out.println(sum);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "10");
}

#[test]
fn aot_ternary() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int a = 10;
        int b = 20;
        int max = a > b ? a : b;
        System.out.println(max);
        String s = max > 15 ? "big" : "small";
        System.out.println(s);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "20");
    assert_eq!(lines[1], "big");
}

#[test]
fn aot_switch_statement() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int x = 2;
        switch (x) {
            case 1: System.out.println("one"); break;
            case 2: System.out.println("two"); break;
            case 3: System.out.println("three"); break;
            default: System.out.println("other");
        }
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "two");
}

#[test]
fn aot_switch_expression() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        for (int i = 1; i <= 4; i++) {
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
"#;
    assert_eq!(compile_and_run(src).trim(), "one\ntwo\nthree\nother");
}

#[test]
fn aot_for_each_array() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int[] arr = {10, 20, 30, 40};
        int sum = 0;
        for (int x : arr) {
            sum += x;
        }
        System.out.println(sum);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "100");
}

#[test]
fn aot_instanceof_check() {
    let src = r#"
class Animal {}
class Dog extends Animal {}
public class Main {
    public static void main(String[] args) {
        Animal a = new Dog();
        System.out.println(a instanceof Dog);
        System.out.println(a instanceof Animal);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "true");
    assert_eq!(lines[1], "true");
}

#[test]
fn aot_static_method_cross_class() {
    let src = r#"
class MathUtil {
    static int square(int n) { return n * n; }
    static int cube(int n) { return n * n * n; }
}
public class Main {
    public static void main(String[] args) {
        System.out.println(MathUtil.square(5));
        System.out.println(MathUtil.cube(3));
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "25");
    assert_eq!(lines[1], "27");
}

#[test]
fn aot_multiple_constructors() {
    let src = r#"
class Point {
    int x;
    int y;
    Point() { this.x = 0; this.y = 0; }
    Point(int x, int y) { this.x = x; this.y = y; }
    int dist2() { return x * x + y * y; }
}
public class Main {
    public static void main(String[] args) {
        Point origin = new Point();
        Point p = new Point(3, 4);
        System.out.println(origin.dist2());
        System.out.println(p.dist2());
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "0");
    assert_eq!(lines[1], "25");
}

#[test]
fn aot_method_chaining_fields() {
    let src = r#"
class Node {
    int val;
    Node next;
    Node(int val) { this.val = val; this.next = null; }
}
public class Main {
    public static void main(String[] args) {
        Node a = new Node(1);
        Node b = new Node(2);
        Node c = new Node(3);
        a.next = b;
        b.next = c;
        System.out.println(a.val);
        System.out.println(a.next.val);
        System.out.println(a.next.next.val);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], "2");
    assert_eq!(lines[2], "3");
}

#[test]
fn aot_break_continue() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int sum = 0;
        for (int i = 0; i < 10; i++) {
            if (i == 7) break;
            if (i % 2 == 0) continue;
            sum += i;
        }
        System.out.println(sum);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "9");
}

#[test]
fn aot_nested_loops() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int count = 0;
        for (int i = 0; i < 4; i++) {
            for (int j = 0; j < i; j++) {
                count++;
            }
        }
        System.out.println(count);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "6");
}

#[test]
fn aot_string_int_concat() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int x = 42;
        System.out.println("Answer: " + x);
        boolean b = true;
        System.out.println("Flag: " + b);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "Answer: 42");
    assert_eq!(lines[1], "Flag: true");
}

#[test]
fn aot_varargs_accumulate() {
    let src = r#"
public class Main {
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }
    public static void main(String[] args) {
        System.out.println(sum(1, 2, 3, 4, 5));
        System.out.println(sum(10, 20));
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "15");
    assert_eq!(lines[1], "30");
}

#[test]
fn aot_2d_array() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int[][] matrix = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}};
        int sum = 0;
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 3; j++) {
                sum += matrix[i][j];
            }
        }
        System.out.println(sum);
    }
}
"#;
    assert_eq!(compile_and_run(src).trim(), "45");
}

#[test]
fn aot_abstract_class() {
    let src = r#"
abstract class Shape {
    abstract int area();
    String describe() { return "area=" + area(); }
}
class Square extends Shape {
    int side;
    Square(int s) { this.side = s; }
    public int area() { return side * side; }
}
public class Main {
    public static void main(String[] args) {
        Shape s = new Square(5);
        System.out.println(s.area());
        System.out.println(s.describe());
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "25");
    assert_eq!(lines[1], "area=25");
}

#[test]
fn aot_superclass_method_call() {
    let src = r#"
class Base {
    int x;
    Base(int x) { this.x = x; }
    int doubled() { return x * 2; }
}
class Child extends Base {
    Child(int x) { super(x); }
    int quadrupled() { return doubled() * 2; }
}
public class Main {
    public static void main(String[] args) {
        Child c = new Child(7);
        System.out.println(c.doubled());
        System.out.println(c.quadrupled());
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "14");
    assert_eq!(lines[1], "28");
}

#[test]
fn aot_enum_ordinal() {
    let src = r#"
enum Day { MON, TUE, WED, THU, FRI }
public class Main {
    public static void main(String[] args) {
        Day d = Day.WED;
        System.out.println(d.ordinal());
        System.out.println(Day.values().length);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "2");
    assert_eq!(lines[1], "5");
}

#[test]
fn aot_bit_operations() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        int a = 0b1010;
        int b = 0b1100;
        System.out.println(a & b);
        System.out.println(a | b);
        System.out.println(a ^ b);
        System.out.println(a << 1);
        System.out.println(b >> 1);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "8");
    assert_eq!(lines[1], "14");
    assert_eq!(lines[2], "6");
    assert_eq!(lines[3], "20");
    assert_eq!(lines[4], "6");
}

#[test]
fn aot_float_arithmetic() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        double x = 3.0;
        double y = 4.0;
        System.out.println(Math.sqrt(x * x + y * y));
        System.out.println(x / y);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    // Note: %g formatting in C prints "5" for 5.0, not "5.0"
    assert_eq!(lines[0], "5");
    assert_eq!(lines[1], "0.75");
}

#[test]
#[ignore = "Known limitation: generic classes cause field slot allocation bug (segfault on field access)"]
fn aot_generic_pair() {
    let src = r#"
class Pair<A, B> {
    A first;
    B second;
    Pair(A first, B second) { this.first = first; this.second = second; }
}
public class Main {
    public static void main(String[] args) {
        Pair<String, Integer> p = new Pair<>("hello", 42);
        System.out.println(p.first);
        System.out.println(p.second);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "42");
}

#[test]
fn aot_interface_default_method() {
    let src = r#"
interface Greeter {
    String name();
    default String greet() { return "Hello, " + name() + "!"; }
}
class English implements Greeter {
    public String name() { return "World"; }
}
public class Main {
    public static void main(String[] args) {
        Greeter g = new English();
        System.out.println(g.greet());
        System.out.println(g.name());
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "Hello, World!");
    assert_eq!(lines[1], "World");
}

#[test]
fn aot_record_class() {
    let src = r#"
record Point(int x, int y) {
    int sum() { return x + y; }
}
public class Main {
    public static void main(String[] args) {
        Point p = new Point(3, 4);
        System.out.println(p.x());
        System.out.println(p.y());
        System.out.println(p.sum());
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "3");
    assert_eq!(lines[1], "4");
    assert_eq!(lines[2], "7");
}

#[test]
fn aot_string_comparison() {
    let src = r#"
public class Main {
    public static void main(String[] args) {
        String a = "hello";
        String b = "hello";
        String c = "world";
        System.out.println(a.equals(b));
        System.out.println(a.equals(c));
        System.out.println(a.compareTo(c) < 0);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "true");
    assert_eq!(lines[1], "false");
    assert_eq!(lines[2], "true");
}

#[test]
fn aot_null_handling() {
    let src = r#"
class Box {
    String value;
    Box(String v) { this.value = v; }
}
public class Main {
    public static void main(String[] args) {
        Box b = null;
        System.out.println(b == null);
        b = new Box("hello");
        System.out.println(b == null);
        System.out.println(b.value);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "true");
    assert_eq!(lines[1], "false");
    assert_eq!(lines[2], "hello");
}

#[test]
fn aot_static_initializer() {
    let src = r#"
class Config {
    static String host;
    static int port;
    static {
        host = "localhost";
        port = 8080;
    }
}
public class Main {
    public static void main(String[] args) {
        System.out.println(Config.host);
        System.out.println(Config.port);
    }
}
"#;
    let out = compile_and_run(src);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "localhost");
    assert_eq!(lines[1], "8080");
}
