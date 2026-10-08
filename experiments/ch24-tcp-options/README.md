# 第 24 章：TCP 选项、MTU 与 PMTUD

## 实验入口

```bash
make experiment EXP=ch24-tcp-options
make experiment EXP=ch24-pmtud
```

## TCP 选项协商

`ch24-tcp-options` 在 TCP/9011 建立短连接，抓包保存为 `captures/ch24-tcp-options.pcap`。

```bash
tcpdump -r captures/ch24-tcp-options.pcap -nn -e -v
```

重点查看 SYN 和 SYN/ACK：

- MSS：对端愿意接收的最大 TCP payload，受接口 MTU 和 IP/TCP 首部长度影响。
- Window Scale：扩大 TCP 窗口字段的有效范围，适合高带宽时延积网络。
- SACK Permitted：允许后续 ACK 报告非连续块，减少丢包时的重复发送。
- Timestamp：用于 RTT 测量和 PAWS 防止旧报文干扰。

这些选项主要在握手时协商，之后数据段按协商结果工作。选项不是独立协议，而是 TCP 首部中的可变长度字段。

## PMTUD / MTU 观察

`ch24-pmtud` 把 client 的 `eth0` MTU 临时设为 1200，再发送 65536 字节 TCP 数据，最后恢复 MTU。抓包文件为 `captures/ch24-pmtud.pcap`。

```bash
tcpdump -r captures/ch24-pmtud.pcap -nn -e -v
```

降低 MTU 后，TCP 通常通过 MSS 协商避免发送超过路径承载能力的 IP 包。经典 IPv4 PMTUD 还可以依靠 DF 包和 ICMP Fragmentation Needed 发现更小的路径 MTU；本 Docker bridge 场景未必稳定产生该 ICMP，因此应同时观察 MSS、IP 总长度、TCP 分段和是否有分片。

## 协议层对照

```text
Ethernet MTU -> IPv4 total length/DF -> TCP MSS -> 应用字节流
```

MSS 是 TCP payload 建议值，MTU 是链路承载的 IP 包大小，两者不要混为一谈。IPv4 总长度还包含 IP 首部；TCP 选项会增大 TCP 首部，从而影响可用 payload。

## 限制

Docker bridge、网卡 offload 和 Linux 内核设置可能让抓包看到的分段与线上物理网卡略有不同。该实验用于理解字段和协商关系，不是精确测量真实高速链路的 PMTUD 性能。
