# 第 18 章：TCP 连接建立与终止

## 实验入口

```bash
make experiment EXP=ch18-tcp-close
make experiment EXP=ch18-tcp-reset
```

## TCP 四次挥手

`ch18-tcp-close` 中 client 发送文本后关闭连接，server 读到 EOF 后退出。抓包文件为 `captures/ch18-tcp-close.pcap`。

```bash
tcpdump -r captures/ch18-tcp-close.pcap -nn -e -v
```

典型顺序：

1. 主动关闭方发送 FIN/ACK，表示本方向不再发送数据。
2. 对端返回 ACK，确认 FIN。
3. 对端完成剩余发送后也发送 FIN/ACK。
4. 主动关闭方返回最后 ACK。

FIN 消耗一个序列号。半关闭很重要：收到 FIN 的一方仍可能继续发送数据，直到它也关闭自己的方向。

## RST

`ch18-tcp-reset` 让 client 连接没有监听者的 TCP/9005。server 内核直接返回 RST，抓包文件为 `captures/ch18-tcp-reset.pcap`。

```bash
tcpdump -r captures/ch18-tcp-reset.pcap -nn -e -v
```

RST 是立即终止或拒绝当前 TCP 状态的控制报文，常见于：

- 目标端口没有监听 socket。
- 收到无法匹配现有连接的报文。
- 应用或内核异常终止连接。

## 状态对应

握手会经过 `SYN-SENT`、`SYN-RECEIVED`、`ESTABLISHED`；正常关闭涉及 `FIN-WAIT`、`CLOSE-WAIT`、`LAST-ACK`、`TIME-WAIT` 等状态。抓包中的 FIN/ACK 序列是状态机在网络上的可见结果，完整状态还可用 `ss -tan` 观察。

## TIME_WAIT

主动关闭方通常进入 TIME_WAIT，以保证旧连接中的延迟报文不会污染后续相同四元组连接，并确保最后 ACK 丢失时仍能重发。它不是“连接还在传数据”，而是 TCP 为可靠终止保留的状态。
