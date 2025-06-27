

pub async fn register_fingerprint_to_capation(rt: tokio::runtime::Handle, common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>) {
    log::debug!("register to capation");

    let (sender, receiver) = tokio::sync::mpsc::channel::<crate::communicate::notifier::NotificationType>(128);
    
    let arcsender = std::sync::Arc::new(sender);
    let notify = crate::communicate::notifier::NotificationSender::new(common.clone(), arcsender.clone());
    
    common.upgrade().unwrap().lock().unwrap().notify = Some(std::sync::Arc::new(std::sync::Mutex::new(notify.clone())));

    let args = std::env::args().collect::<Vec<String>>();
    let mut captain = String::new();

    let mut iter = args.iter().skip_while(|item|!(item.starts_with("-h") || item.starts_with("/h")));
    
    if let Some(_) = iter.next() {
    
        if let Some(addr) = iter.next() {
            log::info!("captain connect addr: {:?}", addr);
            captain = addr.to_owned();
        }
        else {
            log::info!("captain addr not specified, use local address.");
        }
    }
    else {
        log::info!("no -h or /h parameters specified, use local address.");
    }

    rt.spawn(async move {
        let _ = notify.register(receiver, &captain).await;
    });

    let resources = fetch_resource();
    let info = serde_json::to_string(&resources).unwrap();

    log::info!("crew resource: {:?}", info);

    let res = crate::communicate::notifier::NotificationType::Resource(info);
    //channel send
    //first register to captain, then send resource info, captain will broadcast to all crews.
    arcsender.send(res).await.unwrap_or_else(|err | {log::warn!("send local replica resource failed. {:?}", err)});

    let mut fingerprint = crate::fingerprint::gather::SystemInfo::new();

    let weak_common = common.clone();

    let _ = rt.spawn(async move {
        let mut count = 0;
        loop {
            
            let (cpu_usage, memory_used) = crate::fingerprint::gather::SystemInfo::fetch_cpu_and_memory_usage();
            if count <= 12 {
                count += 1;  
                if let Some(common) = weak_common.upgrade() {
                    let common = common.lock().unwrap();
                    if let Some(ref task_manager) = common.tasks {
                        task_manager.lock().unwrap().usage(cpu_usage, memory_used);
                    }
                }
            }
            else {
                count = 0;
                fingerprint.cpu_usage = cpu_usage;
                fingerprint.memory_usage = ((memory_used * 100 as f32 / fingerprint.memory_total) * 1000.0).round() / 1000.0;
                
                let info = serde_json::to_string(&fingerprint).unwrap();
                
                let con = crate::communicate::notifier::NotificationType::Constitution(info);
                match arcsender.send(con).await {
                    Ok(_) => {
                        
                    },
                    Err(err) => {
                        log::warn!("send notification error: {:#?}", err);
                    }
                }   
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
        }
    });
    //handle.await.unwrap();
}

#[cfg(not(feature = "fake"))]
pub fn fetch_resource() -> crate::replica::toolchain::CrewsResource {
    let compilers = crate::replica::toolchain::Property::load_replica_toolchain();
    log::debug!("load local replica toolchain: {:?}", compilers);
    let resources = crate::replica::toolchain::CrewsResource {
        username: crate::fingerprint::gather::SystemInfo::fetch_username(),
        aliasname: crate::fingerprint::gather::SystemInfo::fetch_aliasname(),
        devicename: crate::fingerprint::gather::SystemInfo::fetch_devicename(),
        addr: "".to_string(),
        compiler_versions: compilers
    };
    return resources;
}

#[cfg(feature = "fake")]
pub fn fetch_resource() -> crate::replica::toolchain::CrewsResource {
    let compilers = crate::replica::toolchain::Property::load_replica_toolchain();
    log::debug!("[fake] load local replica toolchain: {:?}", compilers);
    let username = std::env::var("MOCK_USERNAME").unwrap_or_else(|_| "user".to_string());
    let aliasname = std::env::var("MOCK_ALIASNAME").unwrap_or_else(|_| "alias".to_string());
    let devicename = std::env::var("MOCK_DEVICENAME").unwrap_or_else(|_| "device".to_string());
    let resources = crate::replica::toolchain::CrewsResource {
        username: username,
        aliasname: aliasname,
        devicename: devicename,
        addr: "".to_string(),
        compiler_versions: compilers
    };
    return resources;
}