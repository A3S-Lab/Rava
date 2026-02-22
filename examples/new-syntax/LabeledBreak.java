class LabeledBreak {
    public static void main(String[] args) {
        // Test labeled break
        outer:
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 3; j++) {
                if (i == 1 && j == 1) {
                    break outer;
                }
                System.out.println(i * 10 + j);
            }
        }

        System.out.println("---");

        // Test labeled continue
        outer2:
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 3; j++) {
                if (j == 1) {
                    continue outer2;
                }
                System.out.println(i * 10 + j);
            }
        }
    }
}
