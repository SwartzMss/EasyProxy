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

## 环境变量配置

代理的证书路径、监听地址以及认证信息均通过环境变量进行配置。可将这些变量写
入 `.env` 文件并在运行前加载。以下示例给出了全部变量及默认值：

```dotenv
# .env.example
# 使用可信 CA 的证书（占位示例，替换为你的真实域名）
CERT=/etc/letsencrypt/live/proxy.your-domain.example/fullchain.pem
KEY=/etc/letsencrypt/live/proxy.your-domain.example/privkey.pem
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

## 最简单部署（推荐路径）

目标：证书由 EasyProxy 使用；如需 443 端口，对外用 Nginx 做 TCP 直通，避免在 Nginx 再配置证书。

步骤
- 签发与安装证书：参见下文“使用 DNS-01 自动签发（推荐）”。安装完成后，确保：
  - `CERT=/etc/letsencrypt/live/proxy.your-domain.example/fullchain.pem`
  - `KEY=/etc/letsencrypt/live/proxy.your-domain.example/privkey.pem`
- 配置 `.env` 并运行 EasyProxy（监听 8443）：
  - `ADDRESS=0.0.0.0:8443`
  - `cargo run --release`
- 可选：用 Nginx 将 443 直通到 8443（Nginx 不需要证书）
  - 在 `nginx.conf` 或 `conf.d/easyproxy.conf` 添加：
    - `stream {
        server {
          listen 443 reuseport;
          proxy_pass 127.0.0.1:8443;
          proxy_timeout 1h;
          proxy_connect_timeout 10s;
        }
      }`
  - 然后：`sudo nginx -t && sudo systemctl reload nginx`

进阶（可选）
- 不用 Nginx，直接绑定 443：为二进制授予低端口能力或以 root 启动。
  - 授权示例：
    - `sudo setcap 'cap_net_bind_service=+ep' /usr/local/bin/easyproxy`
    - `.env` 中设置 `ADDRESS=0.0.0.0:443` 后直接运行。
  - 注意确保仅授予已安装到固定路径的发布版二进制。

## 使用 DNS-01 自动签发（推荐）

当无法开放 80/443 或需要自动化签发/续期时，建议使用 DNS-01 挑战。下面以 DNSPod + acme.sh 为例（域名均为占位示例，已屏蔽）。同一张证书可在同一域名的多个端口/多个进程中复用。

适用场景
- 不能开放 80/443，但希望自动签发与续期。
- 多端口/多进程共享同一证书与私钥。

步骤（DNSPod + acme.sh）

1) 安装 acme.sh（非 root 安装，无需停机）
- `curl https://get.acme.sh | sh`
- `~/.acme.sh/acme.sh --upgrade --auto-upgrade`

2) 准备 DNSPod API（在 DNSPod 控制台创建 API ID 与 Token）
- `export DP_Id="YOUR_DNSPOD_ID"`
- `export DP_Key="YOUR_DNSPOD_TOKEN"`

3) 设定默认 CA 并注册账户（以 Let's Encrypt 为例）
- `~/.acme.sh/acme.sh --set-default-ca --server letsencrypt`
- `~/.acme.sh/acme.sh --register-account -m you@example.com --server letsencrypt`

4) 使用 DNS-01 签发证书（域名为占位示例）
- 单域名：
  - `~/.acme.sh/acme.sh --issue --dns dns_dp -d proxy.your-domain.example`
- 泛域名（可同时覆盖根域名）：
 - `~/.acme.sh/acme.sh --issue --dns dns_dp -d '*.your-domain.example' -d your-domain.example`

5) 安装到统一路径（供多个进程共享，续期后自动覆盖）
- `sudo mkdir -p /etc/letsencrypt/live/proxy.your-domain.example`
- `sudo ~/.acme.sh/acme.sh --install-cert -d proxy.your-domain.example --key-file /etc/letsencrypt/live/proxy.your-domain.example/privkey.pem --fullchain-file /etc/letsencrypt/live/proxy.your-domain.example/fullchain.pem`

提示：完成服务化后，可在上述命令中追加 `--reloadcmd "<你的重启命令>"`（例如 `systemctl restart easyproxy`），以便续期后自动重载。

6) 在 EasyProxy 中使用
- `.env` 中：
  - `CERT=/etc/letsencrypt/live/proxy.your-domain.example/fullchain.pem`
  - `KEY=/etc/letsencrypt/live/proxy.your-domain.example/privkey.pem`

7) 权限与安全
- 建议让运行用户具备读取私钥的权限（以 Debian/Ubuntu 为例）：
  - `sudo usermod -aG ssl-cert <your-user>`
- 或使用 ACL 精细授权：
  - `sudo setfacl -m u:<your-user>:r /etc/letsencrypt/live/proxy.your-domain.example/privkey.pem`

8) 验证（主机名与证书必须匹配；证书与端口无关）
- `openssl s_client -connect proxy.your-domain.example:8443 -servername proxy.your-domain.example -showcerts`
- 同一张证书可用于 `https://proxy.your-domain.example:8443`、`https://proxy.your-domain.example:9443` 等多个端口。

说明
- 证书绑定“域名”，不绑定端口；同一域名的一张证书可同时用于多个端口与进程。
- 若你的 DNS 提供商不是 DNSPod，可参考 acme.sh 的其它 DNS 插件，命令形式类似（将 `dns_dp` 换为对应插件）。
