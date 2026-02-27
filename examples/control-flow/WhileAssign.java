class Main {
    public static void main(String[] args) {
        String s = "abababab";
        int count = 0, idx = 0;
        // Assignment in body (should work)
        while (idx != -1) {
            System.out.println("idx=" + idx);
            count++;
            idx++;
            idx = s.indexOf("ab", idx);
            if (count > 10) break;  // safety
        }
        System.out.println("count=" + count);
    }
}
