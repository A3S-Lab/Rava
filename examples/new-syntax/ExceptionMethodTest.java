class ExceptionMethodTest {
    public static void main(String[] args) {
        try {
            throw new RuntimeException("something went wrong");
        } catch (RuntimeException e) {
            System.out.println(e.getMessage());
        }

        try {
            throw new IllegalArgumentException("bad arg");
        } catch (Exception e) {
            System.out.println(e.getMessage());
            System.out.println(e.getClass());
        }
    }
}
