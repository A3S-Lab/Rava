class TryCatchReal {
    public static void main(String[] args) {
        try {
            System.out.println("before");
            throw new Exception("error");
        } catch (Exception e) {
            System.out.println("caught");
        }
        System.out.println("after");

        try {
            System.out.println("try");
        } finally {
            System.out.println("finally");
        }

        try {
            try {
                throw new Exception("inner");
            } catch (Exception e) {
                System.out.println("inner caught");
            }
            System.out.println("outer ok");
        } catch (Exception e) {
            System.out.println("outer caught");
        }
    }
}
