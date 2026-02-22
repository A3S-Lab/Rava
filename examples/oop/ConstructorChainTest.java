class Point {
    int x;
    int y;
    String label;

    Point(int x, int y) {
        this.x = x;
        this.y = y;
        this.label = "default";
    }

    Point(int x, int y, String label) {
        this(x, y);
        this.label = label;
    }

    String describe() {
        return "(" + this.x + "," + this.y + ") " + this.label;
    }
}

class ConstructorChainTest {
    public static void main(String[] args) {
        Point p1 = new Point(1, 2);
        Point p2 = new Point(3, 4, "special");
        System.out.println(p1.describe());
        System.out.println(p2.describe());
    }
}
