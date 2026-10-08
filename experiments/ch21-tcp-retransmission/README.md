# 第 21 章：TCP 超时、重传与拥塞控制

## 实验入口

```bash
make experiment EXP=ch21-tcp-retransmission
make experiment EXP=ch21-congestion-control
```

## 超时与重传

`ch21-tcp-retransmission` 在 TCP 连接建立后，通过 router 的 `tc netem` 丢弃 server 方向的 TCP 数据，随后恢复链路。抓包文件为 `captures/ch21-tcp-retransmission.pcap`。

```bash
tcpdump -r captures/ch21-tcp-retransmission.pcap -nn -e -v
```

观察同一个序列号范围重复出现：

1. 原始数据段发送。
2. 数据段或 ACK 被丢弃。
3. 发送端等待 RTO，或因重复 ACK 触发快速重传。
4. 相同序列空间再次出现。
5. 链路恢复后 server 最终收到完整字节流。

RTO 是发送端根据 RTT 估计和波动计算出的重传等待时间。不要把重复序列号误认为重复应用数据；TCP 接收端会丢弃重复字节并继续确认连续范围。

## 拥塞控制大流

`ch21-congestion-control` 发送 4 MiB TCP 数据并抓包到 `captures/ch21-congestion-control.pcap`。它提供观察 cwnd、RTT、ACK、重传和发送速率的样本：

```bash
tcpdump -r captures/ch21-congestion-control.pcap -nn -e -v
# 运行时可在容器内观察连接
ss -ti
```

Reno、NewReno、CUBIC 等算法改变的是拥塞窗口增长和丢包后的退让策略；接收窗口是另一条限制。要做算法对比，需要在相同 RTT、MTU、丢包率和数据量下分别设置内核算法，然后比较吞吐和重传。

## 三个窗口不要混淆

- 接收窗口 `rwnd`：接收端还有多少缓存空间。
- 拥塞窗口 `cwnd`：发送端根据网络拥塞估计允许飞行的数据量。
- 实际发送窗口：通常受两者较小值限制。
