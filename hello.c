int main() {
    int a = 1;
    int b = 2;
    
    if (a > 0) {
        if (b > 1) {
            return 10;
        } else {
            return 20;
        }
    } else {
        if (b < 0) {
            return 30;
        } else {
            return 40;
        }
    }
    
    return 50;
}
