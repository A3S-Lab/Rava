import java.util.Arrays;

class Main {
    public static void main(String[] args) {
        int[] arr = new int[5];
        arr[0] = 5;
        arr[1] = 3;
        arr[2] = 8;
        arr[3] = 1;
        arr[4] = 4;

        System.out.println(Arrays.toString(arr));
        Arrays.sort(arr);
        System.out.println(Arrays.toString(arr));

        int idx = Arrays.binarySearch(arr, 4);
        System.out.println(idx);

        int[] copy = Arrays.copyOf(arr, 3);
        System.out.println(Arrays.toString(copy));

        Arrays.fill(copy, 9);
        System.out.println(Arrays.toString(copy));

        System.out.println(Arrays.equals(arr, arr));
    }
}
