class Counter {
    int count = 0;

    Counter() {
    }

    Counter(int initial) {
        this.count = initial;
    }

    void increment() {
        this.count = this.count + 1;
    }

    void add(int n) {
        this.count = this.count + n;
    }

    int getCount() {
        return this.count;
    }
}

class MultipleClasses {
    public static void main(String[] args) {
        Counter c1 = new Counter();
        Counter c2 = new Counter(10);
        c1.increment();
        c1.increment();
        c1.increment();
        c2.add(5);
        System.out.println(c1.getCount());
        System.out.println(c2.getCount());
    }
}
