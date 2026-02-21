class TryCatch {
    public static void main(String[] args) {
        System.out.println("before try");
        try {
            System.out.println("in try");
        } catch (Exception e) {
            System.out.println("in catch");
        } finally {
            System.out.println("in finally");
        }
        System.out.println("after try");
    }
}
