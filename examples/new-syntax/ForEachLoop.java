class ForEachLoop {
    public static void main(String[] args) {
        int[] nums = new int[5];
        nums[0] = 10;
        nums[1] = 20;
        nums[2] = 30;
        nums[3] = 40;
        nums[4] = 50;
        int sum = 0;
        for (int n : nums) {
            sum = sum + n;
        }
        System.out.println(sum);
    }
}
