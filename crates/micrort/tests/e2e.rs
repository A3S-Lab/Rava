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
