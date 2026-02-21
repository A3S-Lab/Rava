class Node {
    int value = 0;
    Node next = null;

    Node(int value) {
        this.value = value;
    }
}

class LinkedListDemo {
    public static void main(String[] args) {
        Node a = new Node(1);
        Node b = new Node(2);
        Node c = new Node(3);
        a.next = b;
        b.next = c;

        // Walk the list
        Node current = a;
        int sum = 0;
        while (current != null) {
            System.out.println(current.value);
            sum = sum + current.value;
            current = current.next;
        }
        System.out.println(sum);
    }
}
