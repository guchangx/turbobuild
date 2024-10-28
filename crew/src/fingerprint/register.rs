

pub async fn register_fingerprint_to_capation(rt: tokio::runtime::Handle, common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>) {
    
    let notify = crate::communicate::notifier::NotificationSender::new(common);

    let (sender, receiver) = tokio::sync::mpsc::channel::<crate::communicate::notifier::NotificationType>(128);
    
    let sender_ = sender.clone();

    let args = std::env::args().collect::<Vec<String>>();
    let mut captain = String::new();

    let mut iter = args.iter().skip_while(|item| item.starts_with("-h") || item.starts_with("/h"));
    
    if let Some(_) = iter.next() {
        iter.next();
        iter.next();
    
        if let Some(addr) = iter.next() {
            println!("captain addr: {:?}", addr);
            captain = addr.to_owned();
        }
        else {
            
        }
    }
    else {
        println!("no -h or /h parameters specified, use local address.");
    }

    rt.spawn(async move {
        let _ = notify.register(receiver, &captain).await;
    });

    let compilers = crate::replica::toolchain::Property::load_replica_toolchain();
        
    let resources = crate::replica::toolchain::CrewsResource {
        username: crate::fingerprint::gather::SystemInfo::fetch_username(),
        aliasname: crate::fingerprint::gather::SystemInfo::fetch_aliasname(),
        devicename: crate::fingerprint::gather::SystemInfo::fetch_devicename(),
        addr: "".to_string(),
        compiler_versions: compilers
    };
    
    let info = serde_json::to_string(&resources).unwrap();

    let res = crate::communicate::notifier::NotificationType::Resource(info);
    
    sender_.send(res).await.expect("send local replica resource failed.");

    let mut fingerprint = crate::fingerprint::gather::SystemInfo::new();

        
    let rt_ = rt.clone();
    rt.spawn_blocking(move || {
        let _ = rt_.spawn(async move {
            loop {
        
                let (cpu_usage, memory_used) = crate::fingerprint::gather::SystemInfo::fetch_cpu_and_memory_usage();
                
                fingerprint.cpu_usage = cpu_usage;
                fingerprint.memory_usage = ((memory_used * 100 as f32 / fingerprint.memory_total) * 1000.0).round() / 1000.0;
                
                let info = serde_json::to_string(&fingerprint).unwrap();
                
                let con = crate::communicate::notifier::NotificationType::Constitution(info);
                match sender.send(con).await {
                    Ok(_) => {
                       
                    },
                    Err(err) => {
                        println!("send notification error: {:?}", err);
                    }
                }
                
                tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;
            }
        });
    });

}