int main() {
    if (1 > 3)
        return 0;

    if (1 > 2) {
        if (2 > 3 || 3 > 4 && 4 > 5)
            return 3;

        return 1;
    } else
        return 2;

    return 50;
}
