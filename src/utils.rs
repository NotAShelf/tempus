use std::time::Duration;
use crate::Result;

pub fn format_simple_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

pub fn send_notification(name: &str, duration: Duration) -> Result<()> {
    if cfg!(target_os = "linux") {
        let _ = std::process::Command::new("notify-send")
            .args([&format!("{} completed!", name), &format!("Duration: {}", format_simple_duration(duration))])
            .spawn();
    } else if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("osascript")
            .args(["-e", &format!("display notification \"Duration: {}\" with title \"{}\"",
                   format_simple_duration(duration), format!("{} completed!", name))])
            .spawn();
    } else if cfg!(target_os = "windows") {
        let script = format!(
            // Thank you Sky for the PS script. I wouldn't care about it otherwise.
            "powershell -Command \"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); $toastXml = [xml] $template.GetXml(); $toastXml.GetElementsByTagName('text')[0].AppendChild($toastXml.CreateTextNode('{} completed!')) > $null; $toastXml.GetElementsByTagName('text')[1].AppendChild($toastXml.CreateTextNode('Duration: {}')) > $null; $toast = [Windows.UI.Notifications.ToastNotification]::new($toastXml); [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Tempus').Show($toast);\"",
            name,
            format_simple_duration(duration)
        );
        let _ = std::process::Command::new("cmd")
            .args(["/C", &script])
            .spawn();
    }
    Ok(())
}