class RangeIterator {
    int current;
    int end;
    RangeIterator(int current, int end) { this.current = current; this.end = end; }

    boolean hasNext() { return this.current < this.end; }
    int next() {
        int val = this.current;
        this.current = this.current + 1;
        return val;
    }
}

class MyRange {
    int start;
    int end;
    MyRange(int start, int end) { this.start = start; this.end = end; }

    RangeIterator iterator() {
        return new RangeIterator(this.start, this.end);
    }
}

class IteratorTest {
    public static void main(String[] args) {
        // for-each over custom Iterable
        MyRange range = new MyRange(1, 5);
        for (int x : range) {
            System.out.println(x);
        }

        // Iterator used manually
        MyRange r2 = new MyRange(10, 13);
        RangeIterator it = r2.iterator();
        while (it.hasNext()) {
            System.out.println(it.next());
        }
    }
}
