# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   无

2. 此外，我也参考了 **以下资料**，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    - [Linux `struct task_struct`](https://github.com/torvalds/linux/blob/master/include/linux/sched.h)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

# 简答作业

## Q1

正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。请同学们可以自行测试这些内容（运行三个 bad 测例（`ch2b_bad_*.rs`）），描述程序出错行为，同时注意注明你使用的 SBI 及其版本。

```text
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
```

访问非法地址和使用特权指令时，CPU 会触发 trap，程序里的 trap handler 会打印错误日志，如上所示。

本实验使用的 SBI 为 RustSBI，版本为 0.4.1。

## Q2

深入理解 `trap.S` 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题：

### 1. L40

刚进入 `__restore` 时，`sp` 指向什么值？请指出 `__restore` 的两种使用情景。

目前代码里一个 `task` 对应一个内核栈。刚进入 `__restore` 时，`sp` 指向 `TrapContext`，因为进入 `__alltraps` 时就会把 `TrapContext` 压入内核栈。

`__restore` 的第一个使用场景是从普通的 `trap handler` 执行完以后，`.S` 代码 fall through 进入 `__restore`，清理现场，从 `TrapContext` 里记录的信息返回用户态。

另一个使用场景是一个 `task` 的初始化执行：伪造 `TaskContext` 和 `TrapContext`，内核栈跳转运行 `__restore`，`__restore` 从伪造的 `TaskContext` 里跳转进入用户的应用程序入口开始执行代码。

### 2. L43-L48

这几行汇编代码特殊处理了哪些寄存器？这些寄存器的值对于进入用户态有何意义？请分别解释。

```asm
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```

- `32*8(sp): sstatus`：返回到哪个特权级、中断状态如何
- `33*8(sp): sepc`：返回到用户的哪条指令
- 用户栈指针 `2*8(sp): sp`：先暂存进 `sscratch`，随后与内核 `sp` 交换

这几行汇编就是在从 `TrapContext` 里取出以上三个恢复用户态执行现场的重要信息。

### 3. L50-L56

为何跳过了 `x2` 和 `x4`？

```asm
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
LOAD_GP %n
.set n, n+1
.endr
```

- `x2`：用户栈指针 `sp`，不能直接恢复，因为当前内核栈的收尾工作还没做完，直接恢复会丢失剩余的内核栈信息。
- `x4`：线程指针 `tp`，通常在完整 OS 中用于线程局部存储或当前线程指针。目前这个 OS 代码里未使用 `tp`，且注释里说：`# skip tp(x4), application does not use it`

### 4. L60

该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

```asm
csrrw sp, sscratch, sp
```

`csrrw` 是 RISC-V 的 CSR Read and Write：

```text
csrrw rd, csr, rs1
```

可以理解为：

```text
rd = csr;
csr = rs1;
```

对于 L60 的这个 `csrrw sp, sscratch, sp`，就等价于 `swap(sp, sscratch)`。在返回用户态前，把 `sp` 换成用户栈，同时把当前任务的内核栈保存到 `sscratch`，供下一次 trap 使用。

### 5. `__restore` 中的状态切换

`__restore` 中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

`sret` 会从 CPU 当前的 CSR 读取 `sepc`、`sstatus` 等状态，从 S-mode 返回。

### 6. L13

该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

```asm
csrrw sp, sscratch, sp
```

同样是 `swap(sp, sscratch)` 的语义。

执行前，用户态正在运行：

```text
sp       = 用户栈指针
sscratch = 当前任务内核栈顶
```

执行后：

```text
sp       = 当前任务内核栈顶
sscratch = 用户栈指针
```

也就是：

- `sp` 已经从用户栈切换到当前任务的内核栈，内核可以开始执行
- `sscratch` 暂存用户栈指针，等返回用户态时再换回 `sp`

### 7. 从 U 态进入 S 态

从 U 态进入 S 态是哪一条指令发生的？

`ecall` 指令，位于 `user/src/syscall.rs:48`：

```rust
core::arch::asm!(
    "ecall",
    inlateout("x10") args[0] => ret,
    in("x11") args[1],
    in("x12") args[2],
    in("x17") id
);
```

# 实现总结

本次实验实现了 sys_trace 系统调用，支持按地址读写一个用户态字节，并查询当前任务调用指定系统调用的次数。参考 Linux task_struct 保存任务专属状态的思路，将每个任务的统计信息放入 TaskControlBlock，并在系统调用分发入口统一计数，使跟踪调用本身也计入。ch3 测例验证三种功能均通过。
