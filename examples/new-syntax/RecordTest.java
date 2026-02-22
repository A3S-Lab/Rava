record Point(int x, int y) {
}

class RecordTest {
    public static void main(String[] args) {
        Point p = new Point(3, 7);
        System.out.println(p.x());
        System.out.println(p.y());
    }
}
