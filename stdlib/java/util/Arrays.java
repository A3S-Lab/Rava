// Rava stdlib: java.util.Arrays
// Pure Java implementation compiled by Rava itself.

class Arrays {
    static void sort(int[] arr) {
        // Insertion sort (simple, correct for small arrays)
        int n = arr.length;
        int i = 1;
        while (i < n) {
            int key = arr[i];
            int j = i - 1;
            while (j >= 0 && arr[j] > key) {
                arr[j + 1] = arr[j];
                j = j - 1;
            }
            arr[j + 1] = key;
            i = i + 1;
        }
    }

    static void fill(int[] arr, int val) {
        int i = 0;
        while (i < arr.length) {
            arr[i] = val;
            i = i + 1;
        }
    }

    static String toString(int[] arr) {
        if (arr.length == 0) {
            return "[]";
        }
        String s = "[";
        int i = 0;
        while (i < arr.length) {
            if (i > 0) {
                s = s + ", ";
            }
            s = s + arr[i];
            i = i + 1;
        }
        s = s + "]";
        return s;
    }

    static int[] copyOf(int[] src, int newLen) {
        int[] dest = new int[newLen];
        int len = src.length;
        if (newLen < len) {
            len = newLen;
        }
        int i = 0;
        while (i < len) {
            dest[i] = src[i];
            i = i + 1;
        }
        return dest;
    }

    static boolean equals(int[] a, int[] b) {
        if (a.length != b.length) {
            return false;
        }
        int i = 0;
        while (i < a.length) {
            if (a[i] != b[i]) {
                return false;
            }
            i = i + 1;
        }
        return true;
    }

    static int binarySearch(int[] arr, int key) {
        int lo = 0;
        int hi = arr.length - 1;
        while (lo <= hi) {
            int mid = (lo + hi) / 2;
            if (arr[mid] == key) {
                return mid;
            } else if (arr[mid] < key) {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
        return -1;
    }
}
