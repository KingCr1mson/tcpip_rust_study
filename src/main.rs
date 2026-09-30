use std::env;
use std::fs::File;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const CLIENT: &str = "client";
const ROUTER: &str = "router";
const SERVER: &str = "server";
const SERVER_IP: &str = "10.0.2.10";
const ROUTER_CLIENT_IP: &str = "10.0.1.2";
const ROUTER_SERVER_IP: &str = "10.0.2.2";
const UDP_PORT: &str = "9000";
const UDP_UNREACHABLE_PORT: &str = "9001";
const UDP_FRAGMENT_PORT: &str = "9002";
const UDP_FRAGMENT_MTU: &str = "1200";
const UDP_FRAGMENT_SIZE: &str = "3000";
const TCP_HANDSHAKE_PORT: &str = "9003";
const TCP_CLOSE_PORT: &str = "9004";
const UDP_MESSAGE: &str = "netlab udp chapter 11\n";
fn main() {
    let scenario = match env::args().nth(1) {
        Some(value) => value,
        None => {
            eprintln!("用法: netlab <ch09-default-route|ch09-longest-prefix|ch09-missing-route>");
            std::process::exit(2);
        }
    };

    if let Err(error) = run(&scenario) {
        eprintln!("实验失败: {error}");
        std::process::exit(1);
    }
}

fn run(scenario: &str) -> Result<(), String> {
    if scenario == "ch11-udp-unicast" {
        return run_udp_unicast();
    }
    if scenario == "ch11-udp-port-unreachable" {
        return run_udp_port_unreachable();
    }
    if scenario == "ch11-udp-fragmentation" {
        return run_udp_fragmentation();
    }
    if scenario == "ch17-tcp-handshake" {
        return run_tcp_handshake();
    }
    if scenario == "ch18-tcp-close" {
        return run_tcp_close();
    }

    check_scenario(scenario)?;
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes(scenario)?;

    let route = run_command_output("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "get", SERVER_IP]);
    match (scenario == "ch09-missing-route", route) {
        (true, Ok(value)) => return Err(format!("没有路由时却找到了路由: {}", value.trim())),
        (true, Err(_)) => println!("client 的路由选择: 无可用路由"),
        (false, Ok(value)) => println!("client 的路由选择:\n{}", value.trim()),
        (false, Err(error)) => return Err(error),
    }

    let capture_path = format!("captures/{scenario}.pcap");
    println!("抓包文件: {capture_path}");
    let capture_file = File::create(&capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;

    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w -")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let ping_status = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "ping", "-c", "1", "-W", "2", SERVER_IP])
        .status()
        .map_err(|error| format!("无法发送 ping: {error}"))?;

    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    if scenario == "ch09-missing-route" {
        if ping_status.success() {
            return Err("没有路由时 ping 意外成功".to_string());
        }
        println!("验证通过: 没有到 server 的路由，ping 失败");
    } else {
        if !ping_status.success() {
            return Err("存在路由时 ping 失败".to_string());
        }
        println!("验证通过: client 经由 router 到达 server");
    }

    println!("可以使用 tcpdump -r {capture_path} -nn -e 查看抓包");
    Ok(())
}

fn run_udp_unicast() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch11-udp-unicast.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w - udp")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -u -l -p {UDP_PORT}"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 UDP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf '{UDP_MESSAGE}' | nc -u -w 2 {SERVER_IP} {UDP_PORT}"))
        .status()
        .map_err(|error| format!("无法发送 UDP 数据: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 UDP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client.success() {
        return Err("UDP 客户端发送失败".to_string());
    }
    if !received.contains(UDP_MESSAGE.trim()) {
        return Err(format!("UDP 服务端没有收到预期数据: {received:?}"));
    }

    println!("验证通过: UDP 数据经过 router 到达 server");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -X 查看 UDP 数据");
    Ok(())
}
fn run_udp_port_unreachable() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch11-udp-port-unreachable.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w - 'udp or icmp'")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf '{UDP_MESSAGE}' | nc -u -w 2 {SERVER_IP} {UDP_UNREACHABLE_PORT}"))
        .status()
        .map_err(|error| format!("无法发送 UDP 数据: {error}"))?;

    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    if !client.success() {
        return Err("UDP 客户端发送失败".to_string());
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let has_udp = packets.contains("UDP") || packets.contains("udp");
    let has_icmp = packets.contains("unreachable") || packets.contains("ICMP");
    if !has_udp || !has_icmp {
        return Err(format!("抓包没有同时发现 UDP 和 ICMP 端口不可达:\n{packets}"));
    }

    println!("验证通过: 未监听的 UDP 端口返回 ICMP 端口不可达");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e 查看 UDP 和 ICMP");
    Ok(())
}
fn run_udp_fragmentation() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;
    run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "link", "set", "dev", "eth0", "mtu", UDP_FRAGMENT_MTU])?;

    let capture_path = "captures/ch11-udp-fragmentation.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg("timeout 8 tcpdump -i eth0 -U -w - ip")
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -u -l -p {UDP_FRAGMENT_PORT} | wc -c"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 UDP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("head -c {UDP_FRAGMENT_SIZE} /dev/zero | nc -u -w 2 {SERVER_IP} {UDP_FRAGMENT_PORT}"))
        .status()
        .map_err(|error| format!("无法发送大 UDP 数据报: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 UDP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    if !client.success() {
        return Err("UDP 客户端发送失败".to_string());
    }
    let received = String::from_utf8_lossy(&output.stdout);
    if !received.trim().contains(UDP_FRAGMENT_SIZE) {
        return Err(format!("server 没有重组出完整 UDP 数据，收到字节数: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn", "-v"])?;
    let fragment_count = packets.lines().filter(|line| line.contains("offset") || line.contains("Flags [+")).count();
    if fragment_count < 2 {
        return Err(format!("抓包中没有发现至少两个 IPv4 分片:\n{packets}"));
    }

    println!("验证通过: {UDP_FRAGMENT_SIZE} 字节 UDP 数据被分片并在 server 重组");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看分片字段");
    Ok(())
}
fn run_tcp_handshake() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch17-tcp-handshake.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 8 tcpdump -i eth0 -U -w - 'tcp port {TCP_HANDSHAKE_PORT}'"))
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -l -p {TCP_HANDSHAKE_PORT}"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 TCP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf 'tcp handshake chapter 17\\n' | nc -w 2 {SERVER_IP} {TCP_HANDSHAKE_PORT}"))
        .status()
        .map_err(|error| format!("无法启动 TCP 客户端: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 TCP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client.success() {
        return Err("TCP 客户端连接或发送失败".to_string());
    }
    if !received.contains("tcp handshake chapter 17") {
        return Err(format!("TCP 服务端没有收到预期数据: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let has_syn = packets.contains("Flags [S]");
    let has_syn_ack = packets.contains("Flags [S.]");
    let has_ack = packets.contains("Flags [.]");
    if !has_syn || !has_syn_ack || !has_ack {
        return Err(format!("抓包没有发现完整的 TCP 三次握手:\n{packets}"));
    }

    println!("验证通过: TCP 连接建立并观察到 SYN、SYN/ACK、ACK");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看 TCP 首部");
    Ok(())
}
fn run_tcp_close() -> Result<(), String> {
    run_command("docker", &["compose", "up", "-d"])?;
    configure_router()?;
    configure_routes("ch09-default-route")?;

    let capture_path = "captures/ch18-tcp-close.pcap";
    let capture_file = File::create(capture_path)
        .map_err(|error| format!("无法创建抓包文件 {capture_path}: {error}"))?;
    let mut capture = Command::new("docker")
        .args(["compose", "exec", "-T", ROUTER, "sh", "-c"])
        .arg(format!("timeout 8 tcpdump -i eth0 -U -w - 'tcp port {TCP_CLOSE_PORT}'"))
        .stdout(Stdio::from(capture_file))
        .spawn()
        .map_err(|error| format!("无法启动抓包: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let server = Command::new("docker")
        .args(["compose", "exec", "-T", SERVER, "sh", "-c"])
        .arg(format!("timeout 5 nc -l -p {TCP_CLOSE_PORT}"))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 TCP 服务: {error}"))?;

    thread::sleep(Duration::from_secs(1));
    let client = Command::new("docker")
        .args(["compose", "exec", "-T", CLIENT, "sh", "-c"])
        .arg(format!("printf 'tcp close chapter 18\\n' | nc -w 2 {SERVER_IP} {TCP_CLOSE_PORT}"))
        .status()
        .map_err(|error| format!("无法启动 TCP 客户端: {error}"))?;

    let output = server
        .wait_with_output()
        .map_err(|error| format!("等待 TCP 服务失败: {error}"))?;
    capture
        .wait()
        .map_err(|error| format!("等待抓包结束失败: {error}"))?;

    let received = String::from_utf8_lossy(&output.stdout);
    if !client.success() {
        return Err("TCP 客户端关闭连接失败".to_string());
    }
    if !received.contains("tcp close chapter 18") {
        return Err(format!("TCP 服务端没有收到预期数据: {received:?}"));
    }

    let packets = run_command_output("tcpdump", &["-r", capture_path, "-nn"])?;
    let fin_count = packets.matches("Flags [F.").count();
    let ack_count = packets.matches("Flags [.]").count();
    if fin_count < 2 || ack_count < 2 {
        return Err(format!("抓包没有发现完整的 TCP 连接终止过程:\n{packets}"));
    }

    println!("验证通过: TCP 连接关闭时观察到双方 FIN 和 ACK");
    println!("抓包文件: {capture_path}");
    println!("可以使用 tcpdump -r {capture_path} -nn -e -v 查看连接终止");
    Ok(())
}
fn check_scenario(scenario: &str) -> Result<(), String> {
    match scenario {
        "ch09-default-route" | "ch09-longest-prefix" | "ch09-missing-route" | "ch11-udp-unicast" | "ch11-udp-port-unreachable" | "ch11-udp-fragmentation" | "ch17-tcp-handshake" | "ch18-tcp-close" => Ok(()),
        _ => Err(format!("未知实验: {scenario}")),
    }
}

fn configure_router() -> Result<(), String> {
    run_command("docker", &["compose", "exec", "-T", ROUTER, "sysctl", "-w", "net.ipv4.ip_forward=1"])?;
    Ok(())
}

fn configure_routes(scenario: &str) -> Result<(), String> {
    run_command_allow_failure("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "del", "default"]);
    run_command_allow_failure("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "del", "10.0.2.0/24", "via", ROUTER_CLIENT_IP]);
    run_command_allow_failure("docker", &["compose", "exec", "-T", SERVER, "ip", "route", "del", "default"]);

    if scenario != "ch09-missing-route" {
        run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "add", "default", "via", ROUTER_CLIENT_IP])?;
    }
    run_command("docker", &["compose", "exec", "-T", SERVER, "ip", "route", "add", "default", "via", ROUTER_SERVER_IP])?;

    if scenario == "ch09-longest-prefix" {
        run_command("docker", &["compose", "exec", "-T", CLIENT, "ip", "route", "add", "10.0.2.0/24", "via", ROUTER_CLIENT_IP])?;
    }
    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("执行 {program} 失败: {error}"))?;
    if !status.success() {
        return Err(format!("命令失败: {program} {}", args.join(" ")));
    }
    Ok(())
}

fn run_command_allow_failure(program: &str, args: &[&str]) {
    let _ = Command::new(program)
        .args(args)
        .stderr(Stdio::null())
        .status();
}

fn run_command_output(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("执行 {program} 失败: {error}"))?;
    if !output.status.success() {
        return Err(format!("命令失败: {program} {}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
