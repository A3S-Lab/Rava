sealed class Shape permits Circle, Rect {
}

class Circle extends Shape {
    int radius;
    Circle(int r) {
        this.radius = r;
    }
}

class Rect extends Shape {
    int w;
    int h;
    Rect(int w, int h) {
        this.w = w;
        this.h = h;
    }
}

class SealedTest {
    public static void main(String[] args) {
        Circle c = new Circle(5);
        System.out.println(c.radius);

        Rect r = new Rect(3, 4);
        System.out.println(r.w);
        System.out.println(r.h);
    }
}
