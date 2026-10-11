# 第 9 章：IP 选路

## 学习目标

本章实验对应书中的 IP 选路决策流程：目标地址是否匹配某条路由、选择最长网络前缀、没有匹配项时使用默认路由，以及没有路由时如何失败。

## 拓扑

```text
client 10.0.1.10 -- router -- server 10.0.2.10
                       |
              client side 10.0.1.2
              server side 10.0.2.2
```

router 连接两个不同的 `/24` 网络并开启 IPv4 转发。程序只在 router 的 `eth0` 上抓包，因此主要观察 client 发出的二层帧和转发前的 IP 包。

## 实验入口

```bash
make experiment EXP=ch09-default-route
make experiment EXP=ch09-longest-prefix
make experiment EXP=ch09-missing-route
```

抓包查看：

```bash
tcpdump -r captures/ch09-default-route.pcap -nn -e -v
```

也可查看路由表：

```bash
docker compose exec client ip route
docker compose exec router ip route
docker compose exec server ip route
```

## 1. 默认路由

client 没有到 `10.0.2.0/24` 的专用路由，程序配置：

```text
default via 10.0.1.2
```

因此访问 `10.0.2.10` 时，内核把下一跳设为 `10.0.1.2`，Ethernet 目的 MAC 是 router 在 `net_client` 上的 MAC，而不是 server 的 MAC。router 收到帧后重新封装到另一条链路。

IP 层的源地址和目的地址在转发中保持为 `10.0.1.10 -> 10.0.2.10`，但 TTL 减 1；二层源、目的 MAC 在每个链路上都会变化。

## 2. 最长前缀匹配

client 同时拥有：

```text
10.0.2.0/24 via 10.0.1.2
default via 10.0.1.2
```

两个路由都能覆盖 `10.0.2.10`，但 `/24` 比 `/0` 更具体，所以选择 `/24`。本实验让你直接把书中的“最长匹配”对应到 `ip route get 10.0.2.10` 的结果。

## 3. 无路由

程序删除 client 默认路由，也不增加到 server 网络的路由。client 无法决定下一跳，ping 不会产生正常的跨网报文，实验验证失败是预期结果。

## 帧和协议层解读

- Ethernet：决定当前二层网络上的下一跳。
- IPv4：决定端到端的源地址、目的地址和 TTL。
- 路由表：不在帧中，而是发送主机内核作出的决策。
- ICMP Echo：只是用来制造容易识别的 IP 流量，不是本章重点。

## 常见误区

默认路由不是“把目的地址改成网关地址”。它只决定二层下一跳；IP 目的地址仍是最终 server。router 也不会因为转发而修改普通 IPv4 包的源、目的地址。

## 代码与 Docker 操作

三个场景都先经过 `check_scenario()` 白名单，然后进入 `run()` 的通用分支。Rust 依次执行 `docker compose up -d`、在 router 中打开 IPv4 转发，并由 `configure_routes()` 删除旧路由后重新添加路由。默认场景添加 `client ip route add default via 10.0.1.2`；最长前缀场景再添加 `client ip route add 10.0.2.0/24 via 10.0.1.2`；无路由场景不添加 client 默认路由。

`run()` 用 `docker compose exec -T client ip route get 10.0.2.10` 读取内核的选路结果。成功场景启动 router 的 tcpdump，再执行 `ping -c 1 -W 2 10.0.2.10`；无路由场景则要求 ping 失败。`run_command()` 检查外部命令退出码，失败时返回 `Err(String)`，调用方的 `?` 会把错误传回 `main`。
