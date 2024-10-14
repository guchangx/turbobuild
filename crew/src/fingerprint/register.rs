
use super::gather;

pub async fn register_fingerprint_to_capation(rt: tokio::runtime::Handle) {
    
    let notify = crate::communicate::notification::NotificationSender::default();

    let (sender, receiver) = tokio::sync::mpsc::channel::<crate::communicate::notification::NotificationType>(128);

    let _ = notify.register(receiver).await;
    
    let mut fingerprint = crate::fingerprint::gather::SystemInfo::new();
    
    loop {
        
        let (cpu_usage, memory_used) = crate::fingerprint::gather::SystemInfo::fetch_cpu_and_memory_usage();
        
        fingerprint.cpu_usage = cpu_usage;
        fingerprint.memory_usage = ((memory_used * 100 as f32 / fingerprint.memory_total) * 1000.0).round() / 1000.0;
        
        let info = serde_json::to_string(&fingerprint).unwrap();
        
        let con = crate::communicate::notification::NotificationType::Constitution(info);
        let _ = sender.send(con).await;
        
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
}