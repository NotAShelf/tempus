use crate::Result;
use std::time::Duration;
use std::{env, process::Command};

pub fn format_simple_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    match (hours, mins) {
        (h, _) if h > 0 => format!("{}h {}m {}s", hours, mins, secs),
        (_, m) if m > 0 => format!("{}m {}s", mins, secs),
        _ => format!("{}s", secs),
    }
}

pub fn should_use_color() -> bool {
    env::var("NO_COLOR").is_err()
}

#[cfg(target_os = "linux")]
fn send_platform_notification(name: &str, duration_str: &str) -> Result<()> {
    Command::new("notify-send")
        .args([
            &format!("{} completed!", name),
            &format!("Duration: {}", duration_str),
        ])
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn send_platform_notification(name: &str, duration_str: &str) -> Result<()> {
    // This is lifted off Stackoverflow. I do not care if it works, but let me know if it doesn't
    // and I might fix it.
    Command::new("osascript")
        .args([
            "-e",
            &format!(
                "display notification \"Duration: {}\" with title \"{}\"",
                duration_str,
                format!("{} completed!", name)
            ),
        ])
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn send_platform_notification(name: &str, duration_str: &str) -> Result<()> {
    // Thank you Sky for the PS script. I wouldn't care about it otherwise.
    let script = format!(
        "powershell -Command \"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); $toastXml = [xml] $template.GetXml(); $toastXml.GetElementsByTagName('text')[0].AppendChild($toastXml.CreateTextNode('{} completed!')) > $null; $toastXml.GetElementsByTagName('text')[1].AppendChild($toastXml.CreateTextNode('Duration: {}')) > $null; $toast = [Windows.UI.Notifications.ToastNotification]::new($toastXml); [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Tempus').Show($toast);\"",
        name, duration_str
    );
    Command::new("cmd").args(["/C", &script]).spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn send_platform_notification(_name: &str, _duration_str: &str) -> Result<()> {
    // No-op for unsupported platforms
    Ok(())
}

pub fn send_notification(name: &str, duration: Duration) -> Result<()> {
    let duration_str = format_simple_duration(duration);
    send_platform_notification(name, &duration_str)
}
