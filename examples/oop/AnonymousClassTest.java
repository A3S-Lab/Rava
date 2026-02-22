interface Greeting {
    String greet(String name);
}

interface Runnable {
    void run();
}

class AnonymousClassTest {
    public static void main(String[] args) {
        // Anonymous class implementing interface
        Greeting g = new Greeting() {
            public String greet(String name) {
                return "Hello, " + name + "!";
            }
        };
        System.out.println(g.greet("World"));

        // Anonymous class with no-arg method
        Runnable r = new Runnable() {
            public void run() {
                System.out.println("running anonymously");
            }
        };
        r.run();

        // Anonymous class overriding a method
        Greeting formal = new Greeting() {
            public String greet(String name) {
                return "Good day, " + name + ".";
            }
        };
        System.out.println(formal.greet("Sir"));
    }
}
