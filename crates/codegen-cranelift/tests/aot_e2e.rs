//! AOT end-to-end tests: Java source → native binary → run → check output.

use std::process::Command;

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn compile_and_run(java_src: &str) -> String {
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp_dir = std::env::temp_dir().join(format!(
        "rava_aot_{}_{}", std::process::id(), id
    ));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let src_path = tmp_dir.join("Main.java");
    std::fs::write(&src_path, java_src).unwrap();

    let out_path = tmp_dir.join("main_bin");

    // Compile: frontend → RIR → Cranelift → native binary
    let compiler = rava_frontend::Compiler::new();
    let mut module = compiler.compile(java_src, &src_path)
        .expect("compile failed");

    let backend = Box::new(rava_codegen_cranelift::CraneliftBackend::new());
    let aot = rava_aot::AotCompiler::with_default_passes(backend);
    aot.compile(&mut module, &out_path).expect("aot compile failed");

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
