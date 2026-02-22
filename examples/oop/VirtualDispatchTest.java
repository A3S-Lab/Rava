class VirtualDispatchTest {
    static class Animal {
        String speak() { return "..."; }
        String describe() { return "I am an animal that says " + speak(); }
    }

    static class Dog extends Animal {
        String speak() { return "Woof"; }
    }

    static class Cat extends Animal {
        String speak() { return "Meow"; }
    }

    public static void main(String[] args) {
        Dog d = new Dog();
        Cat c = new Cat();
        // speak() is overridden — should dispatch to subclass
        System.out.println(d.speak());
        System.out.println(c.speak());
        // describe() is inherited from Animal — should walk superclass chain
        System.out.println(d.describe());
        System.out.println(c.describe());
    }
}
