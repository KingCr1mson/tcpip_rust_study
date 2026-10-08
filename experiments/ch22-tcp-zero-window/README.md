# 第 22 章：TCP 坚持定时器与零窗口

## 运行

```bash
make experiment EXP=ch22-tcp-zero-window
```

抓包文件：`captures/ch22-tcp-zero-window.pcap`。

```bash
tcpdump -r captures/ch22-tcp-zero-window.pcap -nn -e -v
```

## 原理

接收端通告窗口为 0 时，发送端不能继续发送普通数据，否则会溢出接收缓存。但如果窗口更新 ACK 丢失，双方可能陷入等待：接收端等数据，发送端以为窗口仍为 0。TCP 使用坚持定时器发送 zero-window probe，促使接收端重新通告窗口。

## 本实验做什么

client 向 TCP/9010 发送 4 MiB 数据，server 接收进程输出丢弃，router 抓取完整 TCP 流。抓包中应关注：

- `win 0` 或经过 Window Scale 后的零窗口。
- 窗口重新打开的 ACK。
- 在窗口关闭期间周期性出现的探测段。
- 重传与普通窗口探测的差异。

## 重要限制

当前容器中的 `nc` 和 Linux socket 缓冲区可能让 server 很快读取数据，因此一次运行不一定出现 `win 0`。实验保证的是大数据流和窗口字段可观察；要稳定制造零窗口，通常需要一个故意不读取 socket 的 server、较小的接收缓冲区和足够长的等待时间。

## 概念对应

坚持定时器不是 TCP keep-alive，也不是丢包重传：它专门解决“对端窗口为零或窗口更新可能丢失”的暂停发送问题。
