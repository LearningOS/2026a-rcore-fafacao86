# ch5 Lab3 实验报告

## 实现总结

本次实验将上一章的 `sys_get_time`、`sys_mmap`、`sys_munmap` 迁移到进程结构；实现 `spawn`，直接按 ELF 创建子进程并登记父子关系；实现 `set_priority` 和基于最小堆的 stride 调度。`ch5_usertest` 运行通过。

## 设计

`spawn` 根据用户传入的程序名查找内置 ELF。找到后复用进程初始化流程，创建独立地址空间、用户栈和 trap 上下文，再建立父子关系并加入就绪队列；找不到程序时返回 `-1`。这条路径不复制父进程的用户地址空间。

对于task调度，这里修改了TaskManager里的queue为自定义的Scheduler trait，然后实现了一个基于Stride算法的StrideScheduler impl。
调度器使用 `BinaryHeap` 保存就绪进程。入队时记录当前 stride 作为堆排序键，出队时取 stride 最小的进程，并按 `pass = BIG_STRIDE / priority` 增加该进程的 stride。初始优先级为 16，初始 stride 为 0；`set_priority` 对小于 2 的参数返回 `-1`。目前 `BIG_STRIDE = 1 << 16`，stride 使用 `u64`


## 问答作业：stride 算法深入

### 1. 8 bit 溢出后轮到谁？

`p2` 运行一个时间片后，8 bit 无符号 stride 从 250 加 10 回绕为 4。若直接按整数比较，`4 < 255`，会再次选中 `p2`；因此**实际不会轮到 `p1`**。按照未溢出的逻辑值，`p2` 的 stride 是 260，大于 `p1` 的 255，所以下一次本应选 `p1`。

### 2. 为什么最大与最小 stride 之差不超过 `BigStride / 2`？

所有进程初始 stride 为 0。若所有优先级至少为 2，则每次增加的 `pass = BigStride / priority` 至多为 `BigStride / 2`。


### 3. 考虑溢出时如何比较？

`BinaryHeap` 默认弹出比较结果最大的值，因此要把**逻辑上更小的 stride** 判为 `Greater`。利用两者差距不超过 `BigStride / 2`、且不相等的结果，可以用回绕减法：

```rust
use core::cmp::Ordering;

const BIG_STRIDE: u64 = 1 << 16;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let diff = self.0.wrapping_sub(other.0);
        Some(if diff <= BIG_STRIDE / 2 {
            Ordering::Less
        } else {
            Ordering::Greater
        })
    }
}

impl PartialEq for Stride {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
```

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
无
2. 此外，我也参考了以下资料：

   - https://zhuanlan.zhihu.com/p/405204376
   - [rCore-Tutorial-Guide：chapter5 练习](https://learningos.cn/rCore-Tutorial-Guide/chapter5/4exercise.html)

3. 我会在提交前核对上述交流、参考资料、代码和报告，并如实说明协助情况。我清楚地知道，外部协助可能降低实验难度，影响起评分。

4. 我会遵守课程关于代码来源和交流的规定，妥善保管实验代码，不向他人复制或公开，不提交意图破坏或妨碍评测系统的代码。我清楚地知道，违反课程纪律会受到相应处理。
