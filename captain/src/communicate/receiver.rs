
use std::{error::Error};

use hyper::header::IterMut;


#[allow(non_camel_case_types)]
pub mod notify {
    include!("./../../proto/notify.rs");
}

#[derive(Default, Clone)] 
pub struct NotificationReceiver {
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
    broadcast: std::sync::Arc<tokio::sync::Mutex<Vec<tokio::sync::mpsc::Sender<Result<notify::NotifyResponse, tonic::Status>>>>>,
}

impl NotificationReceiver {

    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        
        let receiver = NotificationReceiver {
            common,
            broadcast: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        };
        return receiver;
    }
    
    pub fn init(&self) {     
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let addr = "0.0.0.0:50051".parse().expect("parse addr failed");
            log::info!("init captain communicate server {}", addr);
            let receiver = NotificationReceiver {
                common: self.common.clone(),
                broadcast: self.broadcast.clone(),
            };
            let server = notify::communicate_server::CommunicateServer::new(receiver);
    
            let result = tonic::transport::Server::builder()
                .add_service(server)
                .serve(addr)
                .await;
            
            match result {
                Ok(_) => {
                    log::debug!("run communicate rpc service end");
                },
                Err(err) => {
                    panic!("run communicate rpc service failed: {}", err);
                }
            }
        });

    }
}

type ResponseStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<notify::NotifyResponse, tonic::Status>> + Send>>;

#[tonic::async_trait]
impl notify::communicate_server::Communicate for NotificationReceiver {

    type notifyStream = ResponseStream;
    async fn notify(&self, request: tonic::Request<tonic::Streaming<notify::NotifyRequest>>) -> Result<tonic::Response<ResponseStream>, tonic::Status> {

        use tokio_stream::StreamExt;
        let addr = request.remote_addr();
        log::debug!("notify request from: {:?}", addr);
        
        let common = self.common.upgrade().expect("upgrade common failed");
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        self.broadcast.lock().await.push(tx.clone());
        
        let broadcast = self.broadcast.clone();

        let _ = tokio::spawn(async move {
            let mut stream = request.into_inner();
            
            while let Some(notification) = stream.next().await {
                match notification {
                    Ok(notification) => {
                        
                        if notify::Type::Register as i32 == notification.r#type {
                            log::debug!("register request: {:?} {:?}", addr, notification);
                            
                            let mut tasker: Vec<crate::roster::crews::Task> = Vec::new();
                            if let Some(addr) = addr {
                                 
                                let common = common.lock().expect("common lock failed");
                                if let Some(tasks) = &common.tasks {
                                    
                                    let mut crew = serde_json::from_str::<crate::roster::crews::CrewRegister>(&notification.message).expect("register request message parse failed");
                                    crew.addr = addr.ip().to_string();
                                    
                                    tasks.lock().expect("roster lock failed").add_from_crew(&crew);
                                    
                                    tasker = tasks.lock().expect("roster lock failed").check("", "", "");
                                }
                            }
                            
                            let tasks = serde_json::to_string(&tasker).unwrap();
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Register as i32,
                                message: tasks,
                                error_code: 0,
                                error_message: "register success".to_string(),
                            };

                            //let _ = tx.send(Ok(reply.clone())).await.expect("tx send failed");
     
                            for (index, crew) in broadcast.lock().await.iter_mut().enumerate() {
                                match crew.send(Ok(reply.clone())).await {
                                    Ok(_) => {},
                                    Err(err) => {

                                        log::warn!("broadcast send failed {}, so delete current index.", err);
                                        broadcast.lock().await.remove(index);             
                                    },
                                }
                            }
                        }
                        else if notify::Type::Unregister as i32 == notification.r#type {
                            log::debug!("unregister request: {:?} {:?}", addr, notification);
                            if let Some(addr) = addr {
                                
                                let common = common.lock().expect("common lock failed");
                                if let Some(roster) = &common.constitutions {
                                    
                                    let mut crew = serde_json::from_str::<crate::roster::crews::CrewRegister>(&notification.message).expect("register request message parse failed");
                                    crew.addr = addr.ip().to_string();
                                    
                                    roster.lock().expect("roster lock failed").remove(crew);
                                }
                            }
                            
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Unregister as i32,
                                message: "unregister success".to_string(),
                                error_code: 0,
                                error_message: "unregister success".to_string(),
                            };
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        }
                        else if notify::Type::Checkresource as i32 == notification.r#type {
                            log::debug!("checkresource request: {:?} {:?}", addr, notification);
                            
                            let mut message = String::new();
                            
                            if let Some(addr) = addr {
                                
                                let common = common.lock().expect("common lock failed");    

                                if let Some(roster) = &common.resources {
                                    
                                    let mut crew = serde_json::from_str::<crate::roster::crews::CrewResource>(&notification.message).expect("register request message parse failed");
                                    crew.addr = addr.ip().to_string();
                                    
                                    let mut resourcelist = roster.lock().expect("roster lock failed");
                                    resourcelist.update(crew);
                                    let res = resourcelist.check("", "", "");
                                    
                                    message = serde_json::to_string(&res).expect("build crew resource json failed.");
                                }
                            }

                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Checkresource as i32,
                                message: message,
                                error_code: 0,
                                error_message: "checkresource success".to_string(),
                            };
                            
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        }
                        else if notify::Type::Keepalive as i32 == notification.r#type {
                            log::debug!("keepalive request: {:?} {:?}", addr, notification);
                            
                            if let Some(addr) = addr {
                                let common = common.lock().expect("common lock failed");
                                if let Some(roster) = &common.constitutions {
                                    
                                    let mut constitution = serde_json::from_str::<crate::roster::crews::CrewConstitution>(&notification.message).expect("keepalive request message parse failed");
                                    constitution.addr = addr.ip().to_string();
                                    roster.lock().expect("roster lock failed").keepalive(constitution);
                                }
                            }
                            
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Keepalive as i32,
                                message: "keepalive success".to_string(),
                                error_code: 0,
                                error_message: "keepalive success".to_string(),
                            };
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        }
                        else {
                            
                        }
                    },
                    Err(err) => {
                        log::debug!("notify client request detail error : {:?}", err);
                        
                        if let Some(source) = err.source() {
                            if let Some(hyper_source) = source.source() {
                                if let Some(inner_source) = hyper_source.source() {
                                    if let Some(err) = inner_source.downcast_ref::<hyper::Error>() {
                                        if let Some(source) = err.source() {
                                            println!("notify client error {:?}, remote {:?}", source.to_string(), addr);

                                            let common = common.lock().expect("common lock failed");    

                                            if let Some(roster) = &common.resources {

                                                let mut resourcelist = roster.lock().expect("roster lock failed");
                                                
                                                resourcelist.remove(&addr.unwrap().ip().to_string(), "", "");
                                            }
           
                                            if let Some(tasks) = &common.tasks {
                                                let mut tasks = tasks.lock().unwrap();
                                                let _ = tasks.remove(&addr.unwrap().ip().to_string(), "", "");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        //TODO: remove hyper error
                    }
                }
            }

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
