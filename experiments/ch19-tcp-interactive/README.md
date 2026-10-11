# 第 19 章：TCP 交互数据流

## 运行

```bash
make experiment EXP=ch19-tcp-interactive
```

抓包文件：`captures/ch19-tcp-interactive.pcap`。

```bash
tcpdump -r captures/ch19-tcp-interactive.pcap -nn -e -v
```

## 场景

client 通过 TCP/9009 发送几个很小的文本片段，server 使用 `nc` 接收。小数据适合观察交互式流量中 ACK、延迟和段合并，而不是吞吐量。

## 观察重点

- 每个方向都有独立的序列号空间。
- ACK 确认的是连续字节，不是“第几个应用消息”。
- 应用的多次小写入不保证产生同样数量的 TCP 段。
- Nagle 算法可能等待已有数据被确认后再发送小段。
- 延迟 ACK 可能暂时不立即回应每个数据段。

本实验的 shell 管道很短，未必稳定触发 Nagle 或延迟 ACK 的明显效果；它的价值是建立“TCP 是字节流”的直觉。需要研究算法差异时，应增加往返延迟、连续小写入次数，并用 `tcpdump` 比较时间戳。

## 协议层对应

```text
应用小写入 -> TCP 序列空间/ACK -> IPv4 -> Ethernet
```

不要把应用换行符、`printf` 调用或 `nc` 的一次读取直接当作 TCP 报文边界。

## 代码与 Docker 操作

该场景调用 `run_tcp_pattern("ch19-tcp-interactive", TCP_INTERACTIVE_PORT, "printf 'a\\nb\\nc\\n'", "交互式小数据流")`。共享函数负责 router 抓 `tcp port 9009`、server 执行 `nc -l -p 9009`，以及 client 执行 `printf 'a\nb\nc\n' | nc -w 2 10.0.2.10 9009`。

`payload` 是交给 client 容器 shell 执行的字符串，`port` 同时决定抓包过滤器和两端 nc 端口。Rust 只检查 client 成功、server stdout 非空并保存 pcap，不假设三个换行对应三个 TCP 段。
