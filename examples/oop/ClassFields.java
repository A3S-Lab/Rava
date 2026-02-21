class Point {
    int x = 0;
    int y = 0;

    Point(int x, int y) {
        this.x = x;
        this.y = y;
    }

    int getX() {
        return this.x;
    }

    int getY() {
        return this.y;
    }

    void move(int dx, int dy) {
        this.x = this.x + dx;
        this.y = this.y + dy;
    }

    String toString() {
        return "(" + this.x + ", " + this.y + ")";
    }
}

class ClassFields {
    public static void main(String[] args) {
        Point p = new Point(3, 4);
        System.out.println(p.getX());
        System.out.println(p.getY());
        System.out.println(p.toString());
        p.move(2, -1);
        System.out.println(p.toString());
    }
}
