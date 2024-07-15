use std::process::Command;

const RESOLV_CONF: &str = "/etc/resolv.conf";
const RESOLV_CONF_BACKUP: &str = "/etc/resolv.conf.corplink.bak";
const SYSTEMD_RESOLVED_CONF: &str = "/etc/systemd/resolved.conf";
const SYSTEMD_RESOLVED_CONF_BACKUP: &str = "/etc/systemd/resolved.conf.corplink.bak";

trait SetSystemDNS {
    fn set(&self, dns: &str) -> Result<(), String>;
}

trait RestoreSystemDNS {
    fn set(&self) -> Result<(), String>;
}

pub async fn set_system_dns(mode: &str, dns: &str) -> Result<(), String> {
    let set_system_dns_actual: Box<dyn SetSystemDNS>;
    match mode {
        #[cfg(unix)]
        "systemd-resolved" => {
            struct Resolvectl;
            impl SetSystemDNS for Resolvectl {
                fn set(&self, dns: &str) -> Result<(), String> {
                    Command::new("cp")
                        .arg(SYSTEMD_RESOLVED_CONF)
                        .arg(SYSTEMD_RESOLVED_CONF_BACKUP)
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;
                    std::fs::write(SYSTEMD_RESOLVED_CONF, format!("[Resolve]\nDNS={}\n", dns))
                        .map_err(|e| format!("Failed to write resolved.conf: {}", e))?;
                    Command::new("systemctl")
                        .arg("restart")
                        .arg("systemd-resolved")
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;
                    Ok(())
                }
            }
            set_system_dns_actual = Box::new(Resolvectl);
        }
        #[cfg(unix)]
        "resolv.conf" => {
            struct ResolvConf;
            impl SetSystemDNS for ResolvConf {
                fn set(&self, dns: &str) -> Result<(), String> {
                    Command::new("cp")
                        .arg(RESOLV_CONF)
                        .arg(RESOLV_CONF_BACKUP)
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;
                    std::fs::write(RESOLV_CONF, format!("nameserver {}\n", dns))
                        .map_err(|e| format!("Failed to write resolv.conf: {}", e))?;
                    Ok(())
                }
            }
            set_system_dns_actual = Box::new(ResolvConf);
        }
        #[cfg(windows)]
        "windows" => {
            struct Windows;
            impl SetSystemDNS for Windows {
                fn set(&self, dns: &str) -> Result<(), String> {
                    let interface_index = get_current_interface()?;
                    let ps_command = format!(
                        "Set-DnsClientServerAddress -InterfaceIndex '{}' -ServerAddresses '{}'",
                        interface_index, dns
                    );
                    Command::new("powershell")
                        .arg("-Command")
                        .arg(&ps_command)
                        .output()
                        .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;
                    Ok(())
                }
            }
            set_system_dns_actual = Box::new(Windows);
        }
        _ => return Err(format!("Unsupported mode: {}", mode)),
    }
    set_system_dns_actual.set(dns)
}

pub async fn restore_system_dns(mode: &str) -> Result<(), String> {
    let restore_system_dns_actual: Box<dyn RestoreSystemDNS>;
    match mode {
        #[cfg(unix)]
        "systemd-resolved" => {
            struct Resolvectl;
            impl RestoreSystemDNS for Resolvectl {
                fn set(&self) -> Result<(), String> {
                    Command::new("cp")
                        .arg(SYSTEMD_RESOLVED_CONF_BACKUP)
                        .arg(SYSTEMD_RESOLVED_CONF)
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;
                    Command::new("systemctl")
                        .arg("restart")
                        .arg("systemd-resolved")
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;
                    Ok(())
                }
            }
            restore_system_dns_actual = Box::new(Resolvectl);
        }
        #[cfg(unix)]
        "resolv.conf" => {
            struct ResolvConf;
            impl RestoreSystemDNS for ResolvConf {
                fn set(&self) -> Result<(), String> {
                    Command::new("cp")
                        .arg(RESOLV_CONF_BACKUP)
                        .arg(RESOLV_CONF)
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;
                    Ok(())
                }
            }
            restore_system_dns_actual = Box::new(ResolvConf);
        }
        #[cfg(windows)]
        "windows" => {
            struct Windows;
            impl RestoreSystemDNS for Windows {
                fn set(&self) -> Result<(), String> {
                    let interface_index = get_current_interface()?;
                    let ps_command = format!(
                        "Set-DnsClientServerAddress -InterfaceIndex '{}' -ResetServerAddresses",
                        interface_index
                    );
                    Command::new("powershell")
                        .arg("-Command")
                        .arg(&ps_command)
                        .output()
                        .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;
                    Ok(())
                }
            }
            restore_system_dns_actual = Box::new(Windows);
        }
        _ => return Err("Unsupported mode".to_string()),
    }
    restore_system_dns_actual.set()
}

#[cfg(windows)]
fn get_current_interface() -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .arg("-Command")
        .arg("Get-NetRoute -DestinationPrefix \"0.0.0.0/0\" | Sort-Object RouteMetric, @{Expression = {$_.ifMetric}; Descending = $true} | Select-Object -First 1 -ExpandProperty InterfaceIndex")
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Command exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let interface_index = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(interface_index)
}
