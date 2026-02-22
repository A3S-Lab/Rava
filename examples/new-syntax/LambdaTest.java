class LambdaTest {
    interface Greeting {
        void greet(String name);
    }

    interface Adder {
        int add(int a, int b);
    }

    public static void main(String[] args) {
        // Lambda with block body
        Greeting g = (name) -> {
            System.out.println("Hello, " + name + "!");
        };
        g.greet("World");

        // Lambda with expression body
        Adder adder = (a, b) -> a + b;
        System.out.println(adder.add(3, 4));

        // Lambda with no params
        Runnable r = () -> System.out.println("running");
        r.run();
    }
}
