# PKU Compiler Lab
北大编译原理实验

doc:https://pku-minic.github.io/online-doc/#/

- [x] lab0: koopaIR库常用接口
- [x] lab1: 简单表达式koopaIR生成
- [x] lab2: 简单表达式riscv生成
- [x] lab3: 表达式(一元表达式, 算术表达式, 逻辑表达式)
- [x] lab4: 常量与变量
- [x] lab5: 语句块和作用域
- [x] lab6: if语句
- [x] lab7: while语句
- [x] lab8: 函数与全局变量
- [ ] lab9: 数组


```shell

# 本地运行命令
cargo run -- -koopa hello.c -o koopair.txt
cargo run -- -riscv hello.c -o riscv.txt

# 本地测试命令
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest ${MODE} -s lv${LEVEL} /root/compiler
```

```cpp 
// 或语句短路求值思路
int a = (b > c) || (c == 0);
/*
 
entry: // 其实这里就相当于lor_lhs_1，只不过可以和entry复用
      %0 = load @b_1
      %1 = load @c_1
      %2 = gt %0, %1
      // 根据比较结果，是否是1，
      // 如果是1直接跳到result_true_1
      // 如果是0就要跳到lor_rhs_1继续计算右侧
      br 
lor_rhs_1:
      %3 = load @c_1
      %4 = ne %3, 0
      // 根据比较结果，是否是1，
      // 如果是1直接跳到result_true_1
      // 如果是0就要跳到result_false_1
      br
result_true_1:
      jump lor_end_1(1) // 无条件跳到lor_end_1,并且告知其逻辑计算结果为1(true)
result_false_1:
      jump lor_end_1(0) // 无条件跳到lor_end_1,并且告知其逻辑计算结果为0(false)            
lor_end_1(i32):
      // 接收计算结果为1(true)还是0(false)
*/
```

```cpp
// 与语句短路求值思路
int b = (a > 20 && a < 10 && a == 15);
    /*
    %entry:
      @b_1 = alloc i32
      %0 = load @a_1
      %1 = gt %0, 20
      %2 = ne %1, 0
      // 如果第一个条件为0，则直接跳转到result_false_2
      // 否则跳转到eval_rhs_2继续计算
      br %2, eval_rhs_2, %result_false_2

    %eval_rhs_2:
      %7 = load @a_1
      %8 = lt %7, 10
      %9 = ne %8, 0
      // 如果第二个条件为0，则直接跳转到result_false_2
      // 否则需要跳转到result_true_2,继续级联判断
      br %9, %result_true_2, %result_false_2

    %result_true_2:
      // 跳转到%land_end_2，并告知前面的式子结果为true
      jump %land_end_2(1)

    %land_end_2(%10: i32):
      // 判断之前来的式子是true 还是 false
      %11 = ne %10, 0
      // 如果前面式子已经是false，直接跳转到false
      // 否则跳转到eval_rhs_1继续下一个表达式的计算
      br %11, %eval_rhs_1, %result_false_1
    */
```