# 第 8 章：Traceroute 与 TTL

## 学习目标

理解 traceroute 如何利用 IP TTL 递减和 ICMP Time Exceeded 发现路径中的路由器。本实验是一个最小 TTL 实验，不实现完整的多跳 traceroute。

## 拓扑与报文路径

```text
client 10.0.1.10 -- 10.0.1.2 router 10.0.2.2 -- server 10.0.2.10
```

client 发送目标为 `10.0.2.10` 的 ICMP Echo Request，并将 TTL 设置为 1。router 收到 IP 包后先将 TTL 减 1，结果为 0，于是丢弃原包并向 client 返回 ICMP Time Exceeded。

## 运行

```bash
make experiment EXP=ch08-traceroute
```

抓包文件：`captures/ch08-traceroute.pcap`。

```bash
tcpdump -r captures/ch08-traceroute.pcap -nn -e -v
```

## 实验流程

1. 启动 client、router、server 三个容器。
2. router 开启 `net.ipv4.ip_forward`，client 和 server 配置默认路由。
3. router 的 client 侧接口启动 tcpdump。
4. client 执行 `ping -t 1 10.0.2.10`。
5. router 发现 TTL 耗尽，生成 ICMP Time Exceeded。
6. Rust 程序检查抓包中是否出现 `time exceeded`。

## 抓包解读

原始请求的 IPv4 首部应包含：

- `src 10.0.1.10`、`dst 10.0.2.10`：最终目标不因 ICMP 错误而改变。
- `ttl 1`：只允许经过一个三层转发点。
- `proto ICMP`：承载 Echo Request。

返回报文应包含：

- `src 10.0.1.2`：产生错误的 router 接口地址。
- `dst 10.0.1.10`：错误通知返回给发送方。
- ICMP 类型 11，代码 0：Time Exceeded in Transit。
- ICMP 负载通常包含触发错误的原 IPv4 首部及其前若干字节，便于 client 匹配原请求。

## 与 traceroute 的对应

完整 traceroute 会依次发送 TTL=1、2、3……的探测包：第一个超时回复暴露第一跳，TTL 足够到达目标后由目标返回 Echo Reply 或 UDP Port Unreachable。本实验只验证第一跳的核心机制。

## 注意事项

Docker bridge、内核版本和 ping 实现可能影响显示字段，但“TTL 耗尽后由 router 返回 ICMP Time Exceeded”是本实验的核心。
