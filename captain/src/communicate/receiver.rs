
use std::error::Error;

#[allow(non_camel_case_types)]
pub mod notify {
    include!("./../../proto/notify.rs");
}

#[derive(Default, Clone)] 
pub struct NotificationReceiver {
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
    broadcaster: std::sync::Arc<tokio::sync::Mutex<Vec<tokio::sync::mpsc::Sender<Result<notify::NotifyResponse, tonic::Status>>>>>,
}

impl NotificationReceiver {

    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        
        let receiver = NotificationReceiver {
            common,
            broadcaster: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        };
        return receiver;
    }
    
    pub fn init(&self) {     
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let addr = "0.0.0.0:18912".parse().expect("parse addr failed");
            log::info!("Starting captain communicate server on {}", addr);
            let receiver = NotificationReceiver {
                common: self.common.clone(),
                broadcaster: self.broadcaster.clone(),
            };
            let server = notify::communicate_server::CommunicateServer::new(receiver);
    
            let result = tonic::transport::Server::builder()
                .add_service(server)
                .serve(addr)
                .await;
            
            match result {
                Ok(_) => {
                    log::info!("Captain communicate RPC service stopped gracefully");
                },
                Err(err) => {
                    log::error!("Captain communicate RPC service failed on {}: {}", addr, err);
                    panic!("run communicate rpc service failed: {} addr: {}", err, addr);
                }
            }
        });

    }

    pub async fn broadcast(&self, response: notify::NotifyResponse) {
        let broadcaster = {
            let mut guard = self.broadcaster.lock().await;
            guard.retain(|tx| !tx.is_closed());
            guard.clone()
        };

        for sender in broadcaster {
            if !sender.is_closed() {
                if let Err(e) = sender.send(Ok(response.clone())).await {
                    log::warn!("Failed to broadcast notification: {:?}", e);
                }
            }
        }
    }
}

type ResponseStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<notify::NotifyResponse, tonic::Status>> + Send>>;

#[tonic::async_trait]
impl notify::communicate_server::Communicate for NotificationReceiver {

    type notifyStream = ResponseStream;
    async fn notify(&self, request: tonic::Request<tonic::Streaming<notify::NotifyRequest>>) -> Result<tonic::Response<ResponseStream>, tonic::Status> {

        use tokio_stream::StreamExt;
        let addr = request.remote_addr();
        let common = self.common.upgrade().expect("upgrade common failed");
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        self.broadcaster.lock().await.push(tx.clone());

        let self_ = self.clone();
        let _ = tokio::spawn(async move {
            let mut stream = request.into_inner();
            
            while let Some(notification) = stream.next().await {
                match notification {
                    Ok(notification) => {
                        if notify::Type::Register as i32 == notification.r#type {
                            let mut tasker: Vec<crate::roster::crews::Task> = Vec::new();
                            if let Some(addr) = addr {
                                let common = common.lock().expect("common lock failed");
                                if let Some(tasks) = &common.tasks {
                                    match serde_json::from_str::<crate::roster::crews::CrewRegister>(&notification.message) {
                                        Ok(mut crew) => {
                                            crew.addr = addr.ip().to_string();
                                            log::info!("Crew registered: {}@{} (cores={}, addr={})",
                                                crew.username, crew.devicename, crew.core, crew.addr);
                                            tasks.lock().expect("roster lock failed").add_from_crew(&crew);
                                            tasker = tasks.lock().expect("roster lock failed").check("", "", "");
                                        }
                                        Err(err) => {
                                            log::error!("Failed to parse register payload from {:?}: {}", addr, err);
                                        }
                                    }
                                }
                            }
                            
                            let tasks = serde_json::to_string(&tasker).unwrap();
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Register as i32,
                                message: tasks,
                                sequence: notification.sequence,
                                error_code: 0,
                                error_message: "register success".to_string(),
                            };

                            self_.broadcast(reply).await;
                        }
                        else if notify::Type::Unregister as i32 == notification.r#type {
                            if let Some(addr) = addr {
                                let common = common.lock().expect("common lock failed");
                                if let Some(roster) = &common.constitutions {
                                    match serde_json::from_str::<crate::roster::crews::CrewRegister>(&notification.message) {
                                        Ok(mut crew) => {
                                            crew.addr = addr.ip().to_string();
                                            log::info!("Crew unregistered: {}@{} (addr={})", crew.username, crew.devicename, crew.addr);
                                            roster.lock().expect("roster lock failed").remove(crew);
                                        }
                                        Err(err) => {
                                            log::error!("Failed to parse unregister payload from {:?}: {}", addr, err);
                                        }
                                    }
                                }
                            }
                            
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Unregister as i32,
                                message: "unregister success".to_string(),
                                sequence: notification.sequence,
                                error_code: 0,
                                error_message: "unregister success".to_string(),
                            };
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        }
                        else if notify::Type::Checkresource as i32 == notification.r#type {
                            let mut message = String::new();
                            
                            if let Some(addr) = addr {
                                let common = common.lock().expect("common lock failed");    
                                if let Some(roster) = &common.resources {
                                    match serde_json::from_str::<crate::roster::crews::CrewResource>(&notification.message) {
                                        Ok(mut crew) => {
                                            if crew.addr.is_empty() {
                                                crew.addr = addr.ip().to_string();
                                            }
                                            log::debug!("Crew resource updated: {}@{} ({})",
                                                crew.username, crew.devicename, crew.addr);
                                            
                                            let mut resourcelist = roster.lock().expect("roster lock failed");
                                            resourcelist.update(crew);
                                            let res = resourcelist.check("", "", "");
                                            message = serde_json::to_string(&res).expect("build crew resource json failed.");
                                        }
                                        Err(err) => {
                                            log::error!("Failed to parse checkresource payload from {:?}: {}", addr, err);
                                        }
                                    }
                                }
                            }

                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Checkresource as i32,
                                message: message,
                                sequence: notification.sequence,
                                error_code: 0,
                                error_message: "checkresource success".to_string(),
                            };
                            
                            self_.broadcast(reply).await;
                        }
                        else if notify::Type::Keepalive as i32 == notification.r#type {
                            if let Some(addr) = addr {
                                let common = common.lock().expect("common lock failed");
                                if let Some(roster) = &common.constitutions {
                                    match serde_json::from_str::<crate::roster::crews::CrewConstitution>(&notification.message) {
                                        Ok(mut constitution) => {
                                            constitution.addr = addr.ip().to_string();
                                            roster.lock().expect("roster lock failed").keepalive(constitution);
                                        }
                                        Err(err) => {
                                            log::warn!("Failed to parse keepalive from {:?}: {}", addr, err);
                                        }
                                    }
                                }
                            }
                            
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Keepalive as i32,
                                message: "keepalive success".to_string(),
                                sequence: notification.sequence,
                                error_code: 0,
                                error_message: "keepalive success".to_string(),
                            };
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        }
                        else {
                            log::warn!("Received unknown notification type {} from {:?}", notification.r#type, addr);
                        }
                    },
                    Err(err) => {
                        let mut cleaned_up = false;
                        if let Some(source) = err.source() {
                            if let Some(hyper_source) = source.source() {
                                if let Some(inner_source) = hyper_source.source() {
                                    if let Some(err) = inner_source.downcast_ref::<hyper::Error>() {
                                        if let Some(_source) = err.source() {
                                            if let Some(addr) = addr {
                                                let ip_str = addr.ip().to_string();
                                                log::info!("Client disconnected, cleaned up crew resources: {}", ip_str);

                                                let common = common.lock().expect("common lock failed");    
                                                if let Some(roster) = &common.resources {
                                                    let mut resourcelist = roster.lock().expect("roster lock failed");
                                                    resourcelist.remove(&ip_str, "", "");
                                                }
               
                                                if let Some(tasks) = &common.tasks {
                                                    let mut tasks = tasks.lock().unwrap();
                                                    let _ = tasks.remove(&ip_str, "", "");
                                                }
                                                cleaned_up = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if !cleaned_up {
                            log::debug!("Client stream error from {:?}: {}", addr, err);
                        }
                    }
                }
            }

            let mut broadcaster = self_.broadcaster.lock().await;
            broadcaster.retain(|tx| {
                !tx.is_closed()
            });
        });

        let response = tokio_stream::wrappers::ReceiverStream::new(rx);
       
        return Ok(tonic::Response::new(Box::pin(response) as ResponseStream));
    }

    async fn report_crews_resource(&self, request: tonic::Request<notify::ReportCrewsResourceRequest>) -> std::result::Result<tonic::Response<notify::ReportCrewsResourceResponse>, tonic::Status> {
        let report_crews_resource = request.into_inner();
        let common = self.common.upgrade().expect("upgrade common failed");
        let common = common.lock().expect("common lock failed");

        for resource in report_crews_resource.resources {
            if let Some(manage) = &common.resources {
                let mut tools = Vec::new();
                for ver in resource.tool_versions {
                    let tool = crate::roster::crews::CompilerVersion {
                        version: ver.version,
                        host:  ver.host.into(),
                        target: ver.target.into(),
                    };
                    tools.push(tool);
                }

                let crew = crate::roster::crews::CrewResource {
                    username: resource.username,
                    aliasname: resource.aliasname,
                    devicename: resource.devicename,
                    addr: resource.addr,
                    compiler_versions: tools,
                };

                manage.lock().expect("roster lock failed").add(crew);
            }
        }
        
        return Ok(tonic::Response::new(notify::ReportCrewsResourceResponse {
            resources: Vec::new(),
            error_code: 0,
            error_message: "".to_string(),
        }));
    }
    
    async fn cancel_crews_resource(&self, request: tonic::Request<notify::ReportCrewsResourceRequest>) -> std::result::Result<tonic::Response<notify::ReportCrewsResourceResponse>, tonic::Status> {
        let report_crews_resource = request.into_inner();
        let common = self.common.upgrade().expect("upgrade common failed");
        let common = common.lock().expect("common lock failed");

        for resource in report_crews_resource.resources {
            if let Some(manage) = &common.resources {
                let mut tools = Vec::new();
                for ver in resource.tool_versions {
                    let tool = crate::roster::crews::CompilerVersion {
                        version: ver.version,
                        host:  ver.host.into(),
                        target: ver.target.into(),
                    };
                    tools.push(tool);
                }

                let crew = crate::roster::crews::CrewResource {
                    username: resource.username,
                    aliasname: resource.aliasname,
                    devicename: resource.devicename,
                    addr: resource.addr,
                    compiler_versions: tools,
                };

                manage.lock().expect("roster lock failed").add(crew);
            }
        }

        return Ok(tonic::Response::new(notify::ReportCrewsResourceResponse {
            resources: Vec::new(),
            error_code: 0,
            error_message: "".to_string(),
        }));
    }

    async fn check_crews_resource(&self, request: tonic::Request<notify::CheckCrewsResourceRequest>) -> std::result::Result<tonic::Response<notify::CheckCrewsResourceResponse>, tonic::Status> {
        let report_crews_resource = request.into_inner();
        let common = self.common.upgrade().expect("upgrade common failed");
        let common = common.lock().expect("common lock failed");

        let mut resources = Vec::new();
        if let Some(manage) = &common.resources {
            let crews = manage.lock().expect("manage lock failed").check(&report_crews_resource.addr, &report_crews_resource.aliasname, &report_crews_resource.username);

            for crew in crews {
                let mut tools = Vec::new();
                for ver in crew.compiler_versions {
                    let tool = crate::communicate::receiver::notify::ToolVersion {
                        version: ver.version,
                        host:  ver.host as i32,
                        target: ver.target as i32,
                    };
                    tools.push(tool);
                }
                
                let res = notify::CrewsResource {
                    username: crew.username,
                    aliasname: crew.aliasname,
                    devicename: crew.devicename,
                    addr: crew.addr,
                    tool_versions: tools,
                };

                resources.push(res);
            }
        }

        return Ok(tonic::Response::new(notify::CheckCrewsResourceResponse {
            resources: resources,
            error_code: 0,
            error_message: "".to_string(),
        }));
    }   
}
