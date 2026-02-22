class Outer {
    static int value = 42;

    static class Inner {
        int x;
        Inner(int x) { this.x = x; }
        int compute() { return this.x + Outer.value; }
    }
}

class StaticInnerTest {
    public static void main(String[] args) {
        Outer.Inner inner = new Outer.Inner(8);
        System.out.println(inner.compute());
        System.out.println(inner.x);
    }
}
