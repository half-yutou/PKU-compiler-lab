int a = 10;
int b = 20;
int main() {
    int a = 10;
    return func(a);// 少了参数居然没有报错...?koopaIR生成时是否应该检查一下
}

// 10个参数
int func(int a, int b, int c, int d, int e, int f, int g, int h, int i, int j) {
    return a + b + c + d + e + f + g + h + i + j;
}
