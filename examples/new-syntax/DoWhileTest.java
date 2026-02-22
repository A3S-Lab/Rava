class DoWhileTest {
    public static void main(String[] args) {
        int i = 0;
        do {
            System.out.println(i);
            i = i + 1;
        } while (i < 3);

        // do-while that runs exactly once
        int x = 100;
        do {
            System.out.println("once");
            x = x + 1;
        } while (x < 100);
    }
}
