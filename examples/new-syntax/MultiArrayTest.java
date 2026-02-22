class MultiArrayTest {
    public static void main(String[] args) {
        // 2D array with new int[3][4]
        int[][] grid = new int[3][4];
        grid[0][0] = 1;
        grid[1][2] = 5;
        grid[2][3] = 9;
        System.out.println(grid[0][0]);
        System.out.println(grid[1][2]);
        System.out.println(grid[2][3]);
        System.out.println(grid[0][1]);

        // Array lengths
        System.out.println(grid.length);
        System.out.println(grid[0].length);
    }
}
