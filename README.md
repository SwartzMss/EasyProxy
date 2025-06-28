# EasyProxy

EasyProxy 旨在实现一个基于 HTTPS 的代理服务器，支持用户名和密码校验，通过 Basic Auth 构造安全接入点，便于在内网或个人开发环境中提供代理。
- 支持 HTTPS 代理
- 通过用户名/密码进行 Basic Authentication 校验
- 可自定义校验规则，灵活控制访问权限
- 适用于需要安全代理或经过中间人服务的情况

## 环境需求

- Rust 1.70 或更高版本（若使用 Rust 实现）

## 适用场景

- 在需要安全控制的局域网中转发 HTTP/HTTPS 请求
- 在自定义脚本或测试环境中提供代理服务
- 当需要通过代理访问外部网络并对请求内容做进一步处理

## 工作流程示例

从客户端到目标服务器，全程只有两层 TLS：

客户端 ↔ 代理 用于保护 CONNECT 报文和凭据

客户端 ↔ 目标站 正常的业务层 HTTPS

```
┌──────────────┐ ① TLS 握手  ┌──────────────┐ ③ 纯 TCP 隧道 ┌──────────────┐
│  浏览器/程序 │════════════▶│   代理:8443  │════════════════▶│ 目标服务器  │
└──────────────┘            │  (Rust)      │                │  (443)      │
        ▲                   └──────────────┘                └──────────────┘
        │ ② CONNECT host:port (含 Basic Auth) 在 TLS 内
        │
        │<═══════════════════════════════════════════════════════════>
               “客户端 ↔ 代理” 这一跳始终被 TLS 加密
```

流程步骤

1. TLS 握手（客户端 ↔ 代理）
2. 客户端把代理当作 HTTPS 代理连接，验证代理证书。
3. 发送 CONNECT 报文（已在 TLS 通道中）
4. CONNECT www.example.com:443 HTTP/1.1
5. Proxy-Authorization: Basic …（用户名/密码）
6. 代理校验凭据并回复 200 Connection Established。
7. 建立透明隧道：代理创建到 www.example.com:443 的 TCP 连接。
8. 从此之后，代理只做字节级转发，不解析任何业务内容。
9. 客户端与目标站自行完成第二层 TLS（正常 HTTPS）。

这样既能保护用户名/密码不裸奔，又保持代理"只转发、不解密"的纯隧道角色。

## 证书生成示例

以下示例均可在 Windows 的 `cmd` 中执行，需要预先安装 OpenSSL。请根据客户端连接时填写的"主机标识"（域名或 IP）选择合适的命令。

### 1. 使用域名

```cmd
openssl req -x509 -newkey rsa:2048 -nodes ^
  -keyout key.pem -out cert.pem -days 365 ^
  -subj "/CN=proxy.example.com" ^
  -addext "subjectAltName=DNS:proxy.example.com"
```

### 2. 使用 IP 地址

```cmd
openssl req -x509 -newkey rsa:2048 -nodes ^
  -keyout key.pem -out cert.pem -days 365 ^
  -subj "/CN=203.0.113.42" ^
  -addext "subjectAltName=IP:203.0.113.42"
```

### 3. 同时支持域名和 IP

```cmd
openssl req -x509 -newkey rsa:2048 -nodes ^
  -keyout key.pem -out cert.pem -days 365 ^
  -subj "/CN=proxy.example.com" ^
  -addext "subjectAltName=DNS:proxy.example.com,IP:203.0.113.42"
```

## 环境变量配置

代理的证书路径、监听地址以及认证信息均通过环境变量进行配置。可将这些变量写
入 `.env` 文件并在运行前加载。以下示例给出了全部变量及默认值：

```dotenv
# .env.example
CERT=cert.pem
KEY=key.pem
USER=user
PASSWD=pass
ADDRESS=0.0.0.0:8443
RUST_LOG=info

# 可选：上游代理设置
# 如果设置了这些变量，EasyProxy 将通过指定的代理连接目标网站
HTTP_PROXY=http://127.0.0.1:7890
HTTPS_PROXY=http://127.0.0.1:7890
```

使用方法：

```bash
cp .env.example .env
cargo run        # 运行代理，自动加载 .env
```
