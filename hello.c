int a = 10;
int b = 20;
int main() {
    int a = 10;// shadow
    return func(a, a, a, a, a, b, b, b, b ,b);
}

// 10个参数(8个内的参数存放于寄存器, 8个以上的参数放于栈中)
int func(int a0, int a1, int a2, int a3, int a4, int a5, int a6, int a7, int a8, int a9) {
    return a0 + a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9;
}
