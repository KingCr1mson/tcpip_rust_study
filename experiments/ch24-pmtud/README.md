# 第 24 章：PMTUD 补充说明

本实验与 TCP 选项实验同属第 24 章，单独保留便于按主题阅读。

```bash
make experiment EXP=ch24-pmtud
tcpdump -r captures/ch24-pmtud.pcap -nn -e -v
```

程序临时把 client 的 MTU 设置为 1200，建立 TCP 连接并发送 65536 字节，再恢复为 1500。抓包重点是：

- SYN 中的 MSS 是否随接口 MTU 改变。
- IPv4 total length 是否超过 1200。
- TCP 是否通过较小 segment 避免 IP 分片。
- 是否出现 DF、ICMP Fragmentation Needed 或重传。

PMTUD 的基本思想是发送带 DF 的 IP 包，让路径中无法转发的大包触发 ICMP“需要分片但 DF 置位”，发送端据此降低路径 MTU。Docker 本地拓扑可能不产生完整的 ICMP 反馈，因此本实验同时作为 MSS/MTU 对照实验。

## 代码与 Docker 操作

`run_pmtud()` 先执行 `docker compose exec -T client ip link set dev eth0 mtu 1200`，再调用共享函数 `run_tcp_pattern("ch24-pmtud", TCP_PMTUD_PORT, "head -c 65536 /dev/zero", "降低 MTU 后的 TCP 传输")`。共享函数在 router 抓 TCP/9012、server 监听、client 用 `head` 生成 65536 字节并通过 `nc` 发送。

实验结束后代码尝试执行 `ip link set dev eth0 mtu 1500` 恢复 client MTU。函数先保存共享函数返回的 `Result<(), String>`，做完恢复后再返回原结果，因此清理动作不会丢失实验错误。
