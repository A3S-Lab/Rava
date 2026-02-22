// Rava stdlib: java.util.Collections
// Pure Java implementation compiled by Rava itself.

class Collections {
    static void reverse(int[] arr) {
        int lo = 0;
        int hi = arr.length - 1;
        while (lo < hi) {
            int tmp = arr[lo];
            arr[lo] = arr[hi];
            arr[hi] = tmp;
            lo = lo + 1;
            hi = hi - 1;
        }
    }

    static int max(int[] arr) {
        int m = arr[0];
        int i = 1;
        while (i < arr.length) {
            if (arr[i] > m) {
                m = arr[i];
            }
            i = i + 1;
        }
        return m;
    }

    static int min(int[] arr) {
        int m = arr[0];
        int i = 1;
        while (i < arr.length) {
            if (arr[i] < m) {
                m = arr[i];
            }
            i = i + 1;
        }
        return m;
    }

    static int frequency(int[] arr, int val) {
        int count = 0;
        int i = 0;
        while (i < arr.length) {
            if (arr[i] == val) {
                count = count + 1;
            }
            i = i + 1;
        }
        return count;
    }
}
