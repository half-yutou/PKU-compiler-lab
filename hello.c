int main() {
    int a = 10;
    int b = 16;
    int c = min(a, b);
    return c;
}

int min(int a, int b) {
    if (a < b) {
        return a;
    } else {
        return b;
    }
}
