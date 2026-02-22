class InheritanceTest {
    static class Animal {
        String name;
        Animal(String name) { this.name = name; }
        String speak() { return "..."; }
    }

    static class Dog extends Animal {
        Dog(String name) { super(name); }
        String speak() { return "Woof"; }
    }

    static class Cat extends Animal {
        Cat(String name) { super(name); }
        String speak() { return "Meow"; }
    }

    public static void main(String[] args) {
        Animal a = new Dog("Rex");
        Dog d = new Dog("Buddy");
        Cat c = new Cat("Whiskers");

        System.out.println(a instanceof Animal);
        System.out.println(a instanceof Dog);
        System.out.println(a instanceof Cat);
        System.out.println(d instanceof Animal);
        System.out.println(c instanceof Animal);
        System.out.println(c instanceof Dog);
    }
}
