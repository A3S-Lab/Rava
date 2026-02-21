class BreakContinue {
    public static void main(String[] args) {
        int sum = 0;
        int i = 0;
        while (i < 20) {
            i++;
            if (i % 2 == 0) continue;
            if (i > 10) break;
            sum = sum + i;
        }
        System.out.println(sum);
    }
}
