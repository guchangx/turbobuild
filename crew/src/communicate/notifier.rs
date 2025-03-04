
#[allow(non_camel_case_types)]
pub mod notify {
    include!("../../proto/notify.rs");
}

#[derive(Default)] 
pub struct NotificationSender {
    common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>,
}

pub enum NotificationType {
    Resource(String),
    Constitution(String),
}

impl NotificationSender {

    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>) ->Self {
        return Self {
            common,
        }
    }

    pub async fn register(&self, mut receiver: tokio::sync::mpsc::Receiver<NotificationType>, addr: &str) -> Result<String, String> {
        use tokio_stream::StreamExt;
        
        let mut ip = "localhost";
        if !addr.is_empty() {
            ip.clone_from(&addr);
        }
        
        match notify::communicate_client::CommunicateClient::connect(format!("http://{}:50051", ip)).await {
            Ok(mut client) => {
                let (tx, rx) = tokio::sync::mpsc::channel(128);
                let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
                match client.notify(request_stream).await {
                    Ok(response) => {
                        
                        let mut response_stream = response.into_inner();
                        let roster = self.common.upgrade().unwrap().lock().unwrap().roster.clone();
                        let tasks = self.common.upgrade().unwrap().lock().unwrap().tasks.clone();
        
                        tokio::spawn(async move {
                            while let Some(stream) = response_stream.next().await {
                                match stream {
                                    Ok(response) => {
                                        if response.r#type == notify::Type::Register as i32 {
                                            let message = response.message.clone();
                                            Self::handle_register_response(tasks.clone(), message.as_str()).await;
                                        }
                                        else if response.r#type == notify::Type::Unregister as i32 {
                                        
                                        }
                                        else if response.r#type == notify::Type::Keepalive as i32 {
        
                                        }
                                        else if response.r#type == notify::Type::Checkresource as i32 {
                                            
                                            let message = response.message.clone();
                                            Self::handle_checkresource_response(roster.clone(), &message).await;
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
                            log::debug!("poll next stream end.");
                        });
                    },
                    Err(status) => {
                        return Err(status.message().to_string());
                    }
                }
                
                let register = self.gather_fingerprint();
                
                let request = notify::NotifyRequest {
                    r#type: notify::Type::Register as i32,
                    message: register,
                };
            
                if let Err(err) = tx.send(request).await {
                    log::error!("notify register error: {:?}", err);
                };
        
                while let Some(notification_type) = receiver.recv().await {
                    match  notification_type {
                        NotificationType::Resource(message) => {
                            let request  = notify::NotifyRequest {
                                r#type: notify::Type::Checkresource as i32,
                                message: message,
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
                        
                            let request = notify::NotifyRequest {
                                r#type: notify::Type::Keepalive as i32,
                                message: message,
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
                
                let unregister_info = self.gather_fingerprint();
                
                let request = notify::NotifyRequest {
                    r#type: notify::Type::Unregister as i32,
                    message: unregister_info,
                };
            
                if let Err(err) = tx.send(request).await {
                    log::error!("notify unregister error: {:?}", err);
                };
        
                return Ok("OK".to_string());
                
            }
            Err(err) => {
                log::error!("can't connect captain host: {}:50051, error: {:?}", ip, err);
                return Err("Err".to_string());
            }
        };
    }

    pub async fn report_resource(&self, resources: Vec<notify::CrewsResource>) {
    
        let mut client = notify::communicate_client::CommunicateClient::connect("http://localhost:50051").await.unwrap();
       
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
        let mut client = notify::communicate_client::CommunicateClient::connect("http://localhost:50051").await.unwrap();

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

    pub async fn handle_checkresource_response(roster: Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>>, message: &str) {
        
        if !message.is_empty() {
            let resources: Vec<crate::replica::toolchain::CrewsResource> = serde_json::from_str(message).expect("serde from json failed.");
            Self::update_crew_resource(roster, &resources).await;
            crate::replica::toolchain::Property::check_resource_and_judge_sync(resources).await;  
            //TODO: time-consuming task, should be done in runtime.
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
            let tasks: Vec<crate::roster::crews::Task> = serde_json::from_str(message).expect("serde from json failed.");
            if let Some(manager) = manager {
                manager.lock().unwrap().add(&tasks);
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
            memory: 31.7,
        };
        let register = serde_json::to_string(&register_info).unwrap();
        return register;
    }
}