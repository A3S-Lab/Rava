class BitwiseOps {
    public static void main(String[] args) {
        int a = 0xFF;
        int b = 0x0F;
        System.out.println(a & b);
        System.out.println(a | b);
        System.out.println(a ^ b);
        System.out.println(1 << 4);
        System.out.println(256 >> 2);
    }
}
