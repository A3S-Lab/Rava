class SwitchExpr {
    public static void main(String[] args) {
        int day = 3;

        // Arrow syntax
        switch (day) {
            case 1 -> System.out.println("Monday");
            case 2 -> System.out.println("Tuesday");
            case 3 -> System.out.println("Wednesday");
            default -> System.out.println("Other");
        }

        // Classic colon syntax still works
        switch (day) {
            case 1:
                System.out.println("Mon");
                break;
            case 3:
                System.out.println("Wed");
                break;
            default:
                System.out.println("???");
                break;
        }
    }
}
