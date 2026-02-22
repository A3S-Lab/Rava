interface Greeter {
    default String greet(String name) {
        return "Hello, " + name + "!";
    }
}

class FriendlyGreeter implements Greeter {
}

class FormalGreeter implements Greeter {
    public String greet(String name) {
        return "Good day, " + name + ".";
    }
}

class DefaultMethodTest2 {
    public static void main(String[] args) {
        FriendlyGreeter f = new FriendlyGreeter();
        System.out.println(f.greet("World"));

        FormalGreeter g = new FormalGreeter();
        System.out.println(g.greet("World"));
    }
}
