

#[allow(non_camel_case_types)]
pub mod notify {
    include!("../../proto/notify.rs");
}

#[derive(Debug, Clone)]
pub struct NotificationSender {
    common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>,
    sender: std::sync::Arc<tokio::sync::mpsc::Sender<NotificationType>>,
}

pub enum NotificationType {
    Resource(String),
    Constitution(String),
}

use tokio_stream::StreamExt;
static PORT:i32 = 18912;
static ADDR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

impl NotificationSender {

    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>, sender: std::sync::Arc<tokio::sync::mpsc::Sender<NotificationType>>) ->Self {
        return Self {
            common,
            sender,
        }
    }

    pub async fn register(&self, mut receiver: tokio::sync::mpsc::Receiver<NotificationType>, addr: &str) -> Result<String, String> {
        use tokio_stream::StreamExt;
        
        let mut ip = "localhost";
        if !addr.is_empty() {
            ip.clone_from(&addr);
        }
        ADDR.set(ip.to_string()).unwrap();

        match notify::communicate_client::CommunicateClient::connect(format!("http://{}:{}", ip, PORT)).await {
            Ok(mut client) => {
                let (tx, rx) = tokio::sync::mpsc::channel(128);
                let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
                match client.notify(request_stream).await {
                    Ok(response) => {
                        
                        let mut response_stream = response.into_inner();
                        let roster = self.common.upgrade().unwrap().lock().unwrap().roster.clone();
                        let tasks = self.common.upgrade().unwrap().lock().unwrap().tasks.clone();
                        let sender = self.sender.clone();
                        tokio::spawn(async move {
                            while let Some(stream) = response_stream.next().await {
                                match stream {
                                    Ok(response) => {
                                        if response.r#type == notify::Type::Register as i32 {
                                            let message = response.message.clone();
                                            Self::handle_register_response(tasks.clone(), message.as_str()).await;
                                        }
                                        else if response.r#type == notify::Type::Unregister as i32 {
                                            let message = response.message.clone();
                                            Self::handle_unregister_response(tasks.clone(), message.as_str()).await;
                                        }
                                        else if response.r#type == notify::Type::Keepalive as i32 {
        
                                        }
                                        else if response.r#type == notify::Type::Checkresource as i32 {
                                            
                                            let message = response.message.clone();
                                            Self::handle_checkresource_response(roster.clone(), &message, sender.clone()).await;
                                        }
                                        else {
                                            
                                        }
                                        log::debug!("notify response: {:?}", response);
                                    }
                                    Err(err) => {
                                        eprintln!("notify response failed: {:?}", err);
                                        break;
                                    }
                                }
                            }
                            log::debug!("captain request stream exist.");
                        });
                    },
                    Err(status) => {
                        return Err(status.message().to_string());
                    }
                }
                
                let register = self.gather_fingerprint();
                
   
                let sequence = std::sync::atomic::AtomicU64::new(0);

                let request = notify::NotifyRequest {
                    r#type: notify::Type::Register as i32,
                    message: register,
                    sequence: sequence.load(std::sync::atomic::Ordering::Relaxed),
                };
            
                if let Err(err) = tx.send(request).await {
                    log::error!("notify register error: {:?}", err);
                };

                while let Some(notification_type) = receiver.recv().await {
                    match  notification_type {
                        NotificationType::Resource(message) => {
                            sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let request  = notify::NotifyRequest {
                                r#type: notify::Type::Checkresource as i32,
                                message: message,
                                sequence: sequence.load(std::sync::atomic::Ordering::Relaxed),
                            };
                            
                            if tx.is_closed() {
                                log::debug!("crew notify tx is closed. so do't send message");
                                break;
                            }
                            else {
                                if let Err(err) = tx.send(request).await {
                                    log::error!("crew notify resource error: {:?}", err);
                                };
                            }
                        },
                        NotificationType::Constitution(message) => {
                            sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let request = notify::NotifyRequest {
                                r#type: notify::Type::Keepalive as i32,
                                message: message,
                                sequence: sequence.load(std::sync::atomic::Ordering::Relaxed),
                            };
                            
                            if tx.is_closed() {
                                log::debug!("crew notify tx is closed. so do't send message");
                                break;
                            }
                            else {
                                if let Err(err) = tx.send(request).await {
                                    log::error!("crew notify constitution error: {:?}", err);
                                    break;
                                }; 
                            }
                        }
                    }
                }
                
                let unregister = self.gather_fingerprint();
                sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let request = notify::NotifyRequest {
                    r#type: notify::Type::Unregister as i32,
                    message: unregister,
                    sequence: sequence.load(std::sync::atomic::Ordering::Relaxed),
                };
            
                if let Err(err) = tx.send(request).await {
                    log::error!("notify unregister error: {:?}", err);
                };
        
                return Ok("OK".to_string());
                
            }
            Err(err) => {
                log::error!("can't connect captain host: {}:{}, error: {:?}", ip, PORT, err);
                return Err("Err".to_string());
            }
        };
    }

    pub async fn report_resource(&self, resources: Vec<notify::CrewsResource>) {
    
        let mut client = notify::communicate_client::CommunicateClient::connect(format!("http://localhost:{}", PORT)).await.unwrap();
       
       let request = notify::ReportCrewsResourceRequest {
            resources
       };
       
        match client.report_crews_resource(request).await {
            Ok(response) => {
                
                let response = response.into_inner();
                log::debug!("response {:?}", response);
            },
            Err(err) => {
                log::debug!("report crew resource. {:?}", err);
            }
        }
    }

    pub async fn check_crews_resource(&self, resource: notify::CrewsResource) -> Vec<notify::CrewsResource> {
        let mut client = notify::communicate_client::CommunicateClient::connect(format!("http://localhost:{}", PORT)).await.unwrap();

        let request = notify::CheckCrewsResourceRequest {
            username: resource.username,
            aliasname: resource.aliasname,
            addr: resource.addr
        };

        match client.check_crews_resource(request).await {
            Ok(response) => {
                
                let response = response.into_inner();
                println!("response {:?}", response);
                if response.error_code == 0 {
                    return response.resources;
                }
                else {
                    return Vec::new();
                }
            }
            Err(err) => {
                log::debug!("report crew resource. {:?}", err);
                return Vec::new();
            }
        }
    }

    pub async fn handle_checkresource_response(roster: Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>>, message: &str, sender: std::sync::Arc<tokio::sync::mpsc::Sender<NotificationType>>) {
        
        if !message.is_empty() {
            let resources: Vec<crate::replica::toolchain::CrewsResource> = serde_json::from_str(message).expect("serde from json failed.");
            Self::update_crew_resource(roster, &resources).await;
            crate::replica::toolchain::Property::check_resource_and_judge_sync(resources, sender).await;
        }
        else {
            log::warn!("check resource response is empty.");
        }
    }

    pub async fn update_crew_resource(roster: Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>>, resouces: &Vec<crate::replica::toolchain::CrewsResource>) {

        if let Some(roster) = roster { 
            roster.lock().unwrap().update(resouces);
        }
    }

    pub async fn handle_register_response(manager: Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::TasksManager>>>, message: &str) {
        if !message.is_empty() {
            match serde_json::from_str::<Vec<crate::roster::crews::Task>>(message) {
                Ok(tasks) => {
                    if let Some(manager) = manager {
                        manager.lock().unwrap().sync(&tasks);
                    }
                }
                Err(err) => {
                    log::error!("Failed to parse register response tasks: {}", err);
                }
            }
        }
    }

    pub async fn handle_unregister_response(manager: Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::TasksManager>>>, message: &str) {
        if !message.is_empty() {
            let tasks = serde_json::from_str::<Vec<crate::roster::crews::Task>>(message)
                .or_else(|_| serde_json::from_str::<crate::roster::crews::Task>(message).map(|t| vec![t]));
            match tasks {
                Ok(tasks) => {
                    if let Some(manager) = manager {
                        manager.lock().unwrap().remove(&tasks);
                    }
                }
                Err(err) => {
                    log::error!("Failed to parse unregister broadcast tasks: {}", err);
                }
            }
        }
    }
    #[cfg(not(feature = "fake"))]
    pub fn gather_fingerprint(&self) -> String {
        let register_info = crate::fingerprint::gather::RegisterInfo::new();
        let register = serde_json::to_string(&register_info).unwrap();
        return register;
    }

    #[cfg(feature = "fake")]
    pub fn gather_fingerprint(&self) -> String {
        let username = std::env::var("MOCK_USERNAME").unwrap_or_else(|_| "user".to_string());
        let devicename = std::env::var("MOCK_DEVICENAME").unwrap_or_else(|_| "device".to_string());
        let register_info = crate::fingerprint::gather::RegisterInfo {
            username: username,
            devicename: devicename,
            addr: "".to_string(),
            passcode: "".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 31.7,
        };
        let register = serde_json::to_string(&register_info).unwrap();
        return register;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use crate::roster::crews::{Task, TasksManager, Usage};

    #[tokio::test]
    async fn test_handle_register_and_unregister_response() {
        let manager = Arc::new(Mutex::new(TasksManager::new()));

        let task1 = Task {
            username: "u1".to_string(),
            devicename: "d1".to_string(),
            addr: "192.168.1.10".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 3,
            max: 10,
            usage: Usage::default(),
            online: true,
        };
        let task2 = Task {
            username: "u2".to_string(),
            devicename: "d2".to_string(),
            addr: "192.168.1.20".to_string(),
            core: 8,
            frequency: vec![3000],
            memory: 16.0,
            running: 1,
            max: 10,
            usage: Usage::default(),
            online: true,
        };

        // 1. Simulate Register broadcast response (full snapshot)
        let reg_json = serde_json::to_string(&vec![task1.clone(), task2.clone()]).unwrap();
        NotificationSender::handle_register_response(Some(manager.clone()), &reg_json).await;

        assert_eq!(manager.lock().unwrap().all().len(), 2);
        assert_eq!(manager.lock().unwrap().alls().len(), 2);

        // 2. Simulate Unregister broadcast response (single node Task in Vec)
        let unreg_task = Task {
            username: "u1".to_string(),
            devicename: "d1".to_string(),
            addr: "192.168.1.10".to_string(),
            online: false,
            ..Default::default()
        };
        let unreg_json = serde_json::to_string(&vec![unreg_task]).unwrap();
        NotificationSender::handle_unregister_response(Some(manager.clone()), &unreg_json).await;

        let locked = manager.lock().unwrap();
        assert_eq!(locked.all().len(), 1); // Only u2 is online
        assert_eq!(locked.alls().len(), 2); // Both slots preserved
        assert!(!locked.alls()[0].online); // u1 is marked offline
        assert_eq!(locked.alls()[0].running, 0); // u1 running reset
        assert!(locked.alls()[1].online); // u2 remains online
    }
}
