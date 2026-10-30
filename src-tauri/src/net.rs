//! 读取 Windows 的「系统代理」设置。
//!
//! **为什么需要这个模块**：`reqwest` 只认 `HTTP_PROXY` / `HTTPS_PROXY` 环境变量，
//! **不读 Windows「Internet 选项」里的代理**。而国内绝大多数人靠 Clash / v2ray
//! 的系统代理上网，那些软件只写注册表，不写环境变量。
//!
//! 结果是：浏览器能打开 wallhaven，我们的程序却连不上，用户看到"图源时好时坏"，
//! 而这个现象会被误判成"图源不稳定"或"作者代码有问题"。
//!
//! 读注册表用 `RegGetValueW` 一个函数就够，不必引入 `winreg` 依赖——
//! 为了读两个值加一个 crate，不划算。

/// 系统代理地址，形如 `http://127.0.0.1:7890`；没开代理返回 `None`。
///
/// 非 Windows 平台恒为 `None`（本项目只发布 Windows，但测试要在任何平台能跑）。
pub fn system_proxy() -> Option<String> {
    platform::proxy_from_registry()
}

/// 从 `ProxyServer` 的字符串里挑出要用的代理。
///
/// 这个值有三种常见形态，都要能处理：
///
/// - `127.0.0.1:7890` —— 单一代理，http/https 共用
/// - `http=127.0.0.1:7890;https=127.0.0.1:7890` —— 按协议分开写
/// - `socks=127.0.0.1:7891` —— 只有 SOCKS
///
/// 优先选 `https=`（我们的请求绝大多数是 https），其次 `http=`，再次整串。
/// 只有 `socks=` 时返回 `None`：reqwest 不带 socks 特性时用不了它，
/// 返回一个用不了的地址只会让真正的错误信息更难读。
pub fn parse_proxy_server(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // 形态一：没有 `=`，整串就是 host:port
    if !raw.contains('=') {
        return normalize_proxy(raw);
    }

    // 形态二/三：按 `;` 分开，找 https / http
    let mut http_entry = None;
    for part in raw.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((scheme, value)) = part.split_once('=') else {
            continue;
        };
        let scheme = scheme.trim().to_ascii_lowercase();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match scheme.as_str() {
            "https" => return normalize_proxy(value),
            "http" if http_entry.is_none() => {
                http_entry = Some(value.to_string());
            }
            // socks= / ftp= 等一律不用
            _ => {}
        }
    }

    http_entry.and_then(|v| normalize_proxy(&v))
}

/// 把 `host:port` / `http://host:port` 统一成带 scheme 的形式。
fn normalize_proxy(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let with_scheme = if value.contains("://") {
        value.to_string()
    } else {
        format!("http://{value}")
    };

    // 用 reqwest 自己的解析器验一遍：不合法就不要，免得后面报一个看不懂的错。
    match reqwest::Url::parse(&with_scheme) {
        Ok(url) if url.host_str().is_some() && url.port().is_some() => Some(with_scheme),
        _ => None,
    }
}

#[cfg(windows)]
mod platform {
    /// `HKEY_CURRENT_USER`，Windows SDK 里的宏值。
    const HKEY_CURRENT_USER: isize = -2147483647i32 as isize;

    /// `RRF_RT_REG_SZ`：只接受 REG_SZ（字符串）。
    const RRF_RT_REG_SZ: u32 = 0x0000_0002;
    /// `RRF_RT_REG_DWORD`：只接受 REG_DWORD。
    const RRF_RT_REG_DWORD: u32 = 0x0000_0010;
    /// `ERROR_SUCCESS`
    const ERROR_SUCCESS: i32 = 0;

    const SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    const VALUE_ENABLE: &str = "ProxyEnable";
    const VALUE_SERVER: &str = "ProxyServer";

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(
            hkey: isize,
            lp_subkey: *const u16,
            lp_value: *const u16,
            dw_flags: u32,
            pdw_type: *mut u32,
            pv_data: *mut core::ffi::c_void,
            pcb_data: *mut u32,
        ) -> i32;
    }

    /// 把 Rust 字符串转成以 0 结尾的 UTF-16，供 Win32 使用。
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn read_dword(value: &str) -> Option<u32> {
        let subkey = wide(SUBKEY);
        let name = wide(value);
        let mut data: u32 = 0;
        let mut size: u32 = std::mem::size_of::<u32>() as u32;
        let mut kind: u32 = 0;

        // SAFETY: 传入的都是我们自己持有的、以 0 结尾的 UTF-16 缓冲区；
        // `pcb_data` 事先写好数据区大小，RegGetValueW 会按它写 `pv_data`。
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                &mut kind,
                (&raw mut data).cast(),
                &mut size,
            )
        };

        (status == ERROR_SUCCESS).then_some(data)
    }

    fn read_string(value: &str) -> Option<String> {
        let subkey = wide(SUBKEY);
        let name = wide(value);

        // 先问需要多大（传 null 数据指针），再按大小读。
        let mut size: u32 = 0;
        let mut kind: u32 = 0;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS || size == 0 {
            return None;
        }

        let mut buf: Vec<u16> = vec![0; (size as usize).div_ceil(2)];
        let mut kind: u32 = 0;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                &mut kind,
                buf.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }

        // 去掉结尾的 0
        while buf.last() == Some(&0) {
            buf.pop();
        }
        String::from_utf16(&buf).ok()
    }

    pub fn proxy_from_registry() -> Option<String> {
        // ProxyEnable 明确为 1 才算"开着"。读到 0 或读不到都当作没开——
        // 宁可直连，也不要拿一个过期的代理配置去发请求。
        if read_dword(VALUE_ENABLE)? != 1 {
            return None;
        }
        let server = read_string(VALUE_SERVER)?;
        super::parse_proxy_server(&server)
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn proxy_from_registry() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_host_port_is_accepted() {
        assert_eq!(
            parse_proxy_server("127.0.0.1:7890").as_deref(),
            Some("http://127.0.0.1:7890")
        );
    }

    #[test]
    fn scheme_is_not_doubled() {
        assert_eq!(
            parse_proxy_server("http://127.0.0.1:7890").as_deref(),
            Some("http://127.0.0.1:7890")
        );
    }

    #[test]
    fn per_scheme_prefers_https() {
        assert_eq!(
            parse_proxy_server("http=127.0.0.1:7890;https=127.0.0.1:7891").as_deref(),
            Some("http://127.0.0.1:7891")
        );
    }

    #[test]
    fn per_scheme_falls_back_to_http() {
        assert_eq!(
            parse_proxy_server("http=127.0.0.1:7890").as_deref(),
            Some("http://127.0.0.1:7890")
        );
    }

    #[test]
    fn socks_only_is_rejected() {
        // 只有 socks 时拒不接受：reqwest 不带 socks 特性，给了也用不了，
        // 反而会让错误信息变成"代理连不上"，掩盖真实原因。
        assert_eq!(parse_proxy_server("socks=127.0.0.1:7891"), None);
    }

    #[test]
    fn empty_and_garbage_are_rejected() {
        assert_eq!(parse_proxy_server(""), None);
        assert_eq!(parse_proxy_server("   "), None);
        assert_eq!(parse_proxy_server("not a proxy"), None);
        // 有 host 没端口也不接受：默认端口的代理几乎不存在，猜一个只会更难排查
        assert_eq!(parse_proxy_server("127.0.0.1"), None);
    }

    /// 手动跑：`cargo test -- --ignored --nocapture system_proxy`
    ///
    /// 结果取决于这台机器有没有开系统代理，所以不能进自动测试——
    /// 但它验证的是 FFI 那一段（`RegGetValueW` 的签名、缓冲区大小），
    /// 那部分没法用纯函数测出来。
    #[test]
    #[ignore]
    fn system_proxy_reads_the_real_registry() {
        println!("system_proxy() = {:?}", system_proxy());
    }
}
