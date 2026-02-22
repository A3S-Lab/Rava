class DefaultMethodTest {
    interface Greeter {
        default String greet(String name) {
            return "Hello, " + name + "!";
        }
    }

    static class FriendlyGreeter implements Greeter {
        // Uses default method from interface
    }

    static class FormalGreeter implements Greeter {
        public String greet(String name) {
            return "Good day, " + name + ".";
        }
    }

    public static void main(String[] args) {
        FriendlyGreeter f = new FriendlyGreeter();
        System.out.println(f.greet("World"));

        FormalGreeter g = new FormalGreeter();
        System.out.println(g.greet("World"));
    }
}
