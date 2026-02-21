class Point {
    int x;
    int y;
    Point(int x, int y) {
        this.x = x;
        this.y = y;
    }
    int sum() {
        return this.x + this.y;
    }
}
class Main {
    public static void main(String[] args) {
        Point p = new Point(3, 7);
        System.out.println(p.x);
        System.out.println(p.y);
        System.out.println(p.sum());
        System.out.println("Point: (" + p.x + ", " + p.y + ")");
    }
}
