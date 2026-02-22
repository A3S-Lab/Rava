class CatchTypeTest {
    public static void main(String[] args) {
        // Test 1: catch specific type
        try {
            throw new RuntimeException("runtime error");
        } catch (IllegalArgumentException e) {
            System.out.println("wrong catch");
        } catch (RuntimeException e) {
            System.out.println("caught runtime: " + e);
        } catch (Exception e) {
            System.out.println("wrong catch 2");
        }

        // Test 2: catch parent type
        try {
            throw new RuntimeException("child");
        } catch (Exception e) {
            System.out.println("caught by parent: " + e);
        }

        // Test 3: no matching catch — propagates to outer
        try {
            try {
                throw new Exception("checked");
            } catch (RuntimeException e) {
                System.out.println("wrong: runtime");
            }
        } catch (Exception e) {
            System.out.println("outer caught: " + e);
        }
    }
}
