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

fn check_scenario(scenario: &str) -> Result<(), String> {
    match scenario {
        "ch09-default-route" | "ch09-longest-prefix" | "ch09-missing-route" => Ok(()),
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
