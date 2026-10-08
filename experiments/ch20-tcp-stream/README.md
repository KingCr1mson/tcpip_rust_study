# 第 20 章：TCP 成块数据流

## 运行

```bash
make experiment EXP=ch20-tcp-stream
```

client 向 server 发送 1 MiB 数据，抓包保存到 `captures/ch20-tcp-stream.pcap`。

```bash
tcpdump -r captures/ch20-tcp-stream.pcap -nn -e -v
```

## 实验原理

TCP 把连续字节放入发送窗口，允许多个数据段在没有逐段等待应用确认的情况下飞行。接收端通过 ACK 告诉发送端“截至哪个序列号之前的字节已经连续收到”，并在窗口字段中公布剩余接收能力。

## 帧解读

- `seq x:y` 表示该段携带的字节范围 `[x, y)`。
- `ack z` 表示接收方下一个期望的字节是 z。
- `win` 是接收窗口广告，实际值可能受 Window Scale 影响。
- 连续数据段通常具有相邻或重叠的序列空间。
- ACK 可能累计确认多个段，不一定每个数据段对应一个 ACK。

## 慢启动联系

新连接开始时拥塞窗口较小，随着 ACK 到达逐渐扩大；实际增长还受接收窗口、MSS、内核算法和 Docker 虚拟链路影响。这个实验主要展示“成块发送和滑动窗口”，不能仅凭一次短流量精确重建完整 CUBIC/Reno 曲线。

## 接收验证

Rust 控制程序让 server 统计收到的字节数，必须得到 `1048576`，因此抓包中的 TCP 分段最终在接收端重组为完整字节流。
