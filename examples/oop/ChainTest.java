class Foo {
    String name;
    Foo(String name) { this.name = name; }
    String getName() { return this.name; }
}

class ChainTest {
    static Foo foo = new Foo("hello");

    public static void main(String[] args) {
        // Chain: static field access + method call
        System.out.println(ChainTest.foo.getName());
    }
}
