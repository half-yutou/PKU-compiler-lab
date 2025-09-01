# PKU Compiler Lab
北大编译原理实验

doc:https://pku-minic.github.io/online-doc/#/

- [x] lab0: koopaIR库常用接口
- [x] lab1: 简单表达式koopaIR生成
- [x] lab2: 简单表达式riscv生成
- [x] lab3: 表达式(一元表达式, 算术表达式, 逻辑表达式)
- [x] lab4: 常量与变量
- [ ] lab5: 语句块和作用域
- [ ] lab6: if语句
- [ ] lab7: while语句
- [ ] lab8: 函数
- [ ] lab9: 数组


```shell

# 本地运行命令
cargo run -- -koopa hello.c -o koopair.txt
cargo run -- -riscv hello.c -o riscv.txt

# 本地测试命令
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest ${MODE} -s lv${LEVEL} /root/compiler
```