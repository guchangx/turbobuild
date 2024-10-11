use std::error::Error;


#[allow(non_camel_case_types)]
pub mod notify {
    include!("./../../proto/notify.rs");
}

#[derive(Default, Clone)] 
pub struct NotificationReceiver {
    common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>,
}

impl NotificationReceiver {

    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::common::Common>>) -> Self {
        
        let receiver = NotificationReceiver {
            common,
        };
        return receiver;
    }
    
    pub fn init(&self) {     
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            println!("init communicate server");
            let addr = "127.0.0.1:50051".parse().expect("parse addr failed");
            let receiver = NotificationReceiver {
                common: self.common.clone(),
            };
            let server = notify::communicate_server::CommunicateServer::new(receiver);
    
            let result = tonic::transport::Server::builder()
                .add_service(server)
                .serve(addr)
                .await;
            
            match result {
                Ok(_) => {
                    println!("run communicate rpc service end");
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
        println!("notify request: {:?}", request);
        
        use tokio_stream::StreamExt;
        let remote_addr = request.remote_addr();
        println!("notify request from: {:?}", remote_addr);
        
        let common = self.common.upgrade().expect("upgrade common failed");

        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        
        let _ = tokio::spawn(async move {
            let mut stream = request.into_inner();
            while let Some(notification) = stream.next().await {
                match notification {
                    Ok(notification) => {
                        
                        if notify::Type::Register as i32 == notification.r#type {
                            println!("register request: {:?}", notification);
                            
                            if let Some(addr) = remote_addr {
                                 
                                let common = common.lock().expect("common lock failed");
                                if let Some(roster) = &common.constitutions {
                                    
                                    let crew = crate::roster::crews::CrewConstitution {
                                        username: "".to_string(),
                                        aliasname: "".to_string(),
                                        role: 1,
                                        addr: addr.to_string(),
                                        cores: 4,
                                        status: 1,
                                        action: 1,
                                        memory: 32,
                                        operating_system: "".to_string(),
                                        cpu_frequency: 1.0,
                                        cpu_load: 1.0,
                                        description: "".to_string(),
                                    };
                                    
                                    roster.lock().expect("roster lock failed").add(crew);
                                }
                            }

                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Register as i32,
                                message: "register success".to_string(),
                                error_code: 0,
                                error_message: "".to_string(),
                            };
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        } 
                        else if notify::Type::Unregister as i32 == notification.r#type {
                            println!("unregister request: {:?}", notification);
                            if let Some(addr) = remote_addr {
                                
                                let common = common.lock().expect("common lock failed");
                                if let Some(roster) = &common.constitutions {

                                    let crew = crate::roster::crews::CrewConstitution {
                                        username: "".to_string(),
                                        aliasname: "".to_string(),
                                        role: 1,
                                        addr: addr.to_string(),
                                        cores: 4,
                                        status: 1,
                                        action: 1,
                                        memory: 32,
                                        operating_system: "".to_string(),
                                        cpu_frequency: 1.0,
                                        cpu_load: 1.0,
                                        description: "".to_string(),
                                    };
                                    
                                    roster.lock().expect("roster lock failed").remove(crew);    
                                }
                            }
                            
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Unregister as i32,
                                message: "unregister success".to_string(),
                                error_code: 0,
                                error_message: "".to_string(),
                            };
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        } 
                        else if notify::Type::Keepalive as i32 == notification.r#type {
                            println!("keepalive request: {:?}", notification);
                            
                            if let Some(addr) = remote_addr {
                                let common = common.lock().expect("common lock failed");
                                if let Some(roster) = &common.constitutions {

                                    let crew = crate::roster::crews::CrewConstitution {
                                        username: "".to_string(),
                                        aliasname: "".to_string(),
                                        role: 1,
                                        addr: addr.to_string(),
                                        cores: 4,
                                        status: 1,
                                        action: 1,
                                        memory: 32,
                                        operating_system: "".to_string(),
                                        cpu_frequency: 1.0,
                                        cpu_load: 1.0,
                                        description: "".to_string(),
                                    };
                                    
                                    roster.lock().expect("roster lock failed").keepalive(crew);    
                                }
                            }
                            
                            let reply = notify::NotifyResponse {
                                r#type: notify::Type::Keepalive as i32,
                                message: "keepalive success".to_string(),
                                error_code: 0,
                                error_message: "".to_string(),
                            };
                            let _ = tx.send(Ok(reply)).await.expect("tx send failed");
                        }
                        else {
                            
                        }
                    },
                    Err(err) => {
                        
                        println!("notify request error: {:?}", err);
                                     
                        if let Some(source) = err.source() {
                            if let Some(err) = source.downcast_ref::<hyper::Error>() {
                                if let Some(err) = err.source() {
                                    if let Some(err) = err.downcast_ref::<std::io::Error>() {
                                        if err.kind() == std::io::ErrorKind::ConnectionReset {
                                            println!("connection reset");
                                        }
                                    }
                                }
                            }
                        }
                        //TODO: remove hyper error
           
                        
                        if let Some(addr) = remote_addr {
                                
                            let common = common.lock().expect("common lock failed");
                            if let Some(roster) = &common.constitutions {

                                let crew = crate::roster::crews::CrewConstitution { 
                                    username: "".to_string(),
                                    aliasname: "".to_string(),
                                    role: 1,
                                    addr: addr.to_string(),
                                    cores: 4,
                                    status: 1,
                                    action: 1,
                                    memory: 32,
                                    operating_system: "".to_string(),
                                    cpu_frequency: 1.0,
                                    cpu_load: 1.0,
                                    description: "".to_string(),
                                };
                                
                                roster.lock().expect("roster lock failed").remove(crew);    
                            }
                        }
                    }
                }
            }
        });

        let response = tokio_stream::wrappers::ReceiverStream::new(rx);
       
        return Ok(tonic::Response::new(Box::pin(response) as ResponseStream));
    }
    async fn unregister(&self, request: tonic::Request<notify::NotifyRequest>) -> Result<tonic::Response<notify::NotifyResponse>, tonic::Status> {
        println!("register request: {:?}", request);

        let reply = notify::NotifyResponse {
            r#type: notify::Type::Register as i32,
            message: "register success".to_string(),
            error_code: 0,
            error_message: "".to_string(),
        };

        Ok(tonic::Response::new(reply))
    }

    async fn report_crews_resource(&self, request: tonic::Request<notify::ReportCrewsResourceRequest>) -> std::result::Result<tonic::Response<notify::ReportCrewsResourceResponse>, tonic::Status> {
        
        let report_crews_resource = request.into_inner();
        let common = self.common.upgrade().expect("upgrade common failed");
        let common = common.lock().expect("common lock failed");

        if let Some(resource) = &common.resources {
            
            let crew = crate::roster::crews::CrewResource {
                username: report_crews_resource.username,
                aliasname: report_crews_resource.aliasname,
                addr: report_crews_resource.addr,
                winkits_includes_path: report_crews_resource.winkits_includes_path.iter().map(|item| std::ffi::OsString::from(item)).collect(),
                compiler_path: std::ffi::OsString::from(report_crews_resource.compiler_path),
                msvc_includes_path: std::ffi::OsString::from(report_crews_resource.msvc_includes_path),
                msvc_version: report_crews_resource.msvc_version,
            };
            
            resource.lock().expect("roster lock failed").add(crew);
        }
        
        return Ok(tonic::Response::new(notify::ReportCrewsResourceResponse {
            error_code: 0,
            error_message: "".to_string(),
        }));
    }
    
    async fn cancel_crews_resource(&self, request: tonic::Request<notify::ReportCrewsResourceRequest>) -> std::result::Result<tonic::Response<notify::ReportCrewsResourceResponse>, tonic::Status> {
        let report_crews_resource = request.into_inner();
        let common = self.common.upgrade().expect("upgrade common failed");
        let common = common.lock().expect("common lock failed");

        if let Some(resource) = &common.resources {
            
            let crew = crate::roster::crews::CrewResource {
                username: report_crews_resource.username,
                aliasname: report_crews_resource.aliasname,
                addr: report_crews_resource.addr,
                winkits_includes_path: report_crews_resource.winkits_includes_path.iter().map(|item| std::ffi::OsString::from(item)).collect(),
                compiler_path: std::ffi::OsString::from(report_crews_resource.compiler_path),
                msvc_includes_path: std::ffi::OsString::from(report_crews_resource.msvc_includes_path),
                msvc_version: report_crews_resource.msvc_version,
            };
            
            resource.lock().expect("roster lock failed").remove(crew);
        }
        
        return Ok(tonic::Response::new(notify::ReportCrewsResourceResponse {
            error_code: 0,
            error_message: "".to_string(),
        }));
    } 
    async fn check_crews_resource(&self, request: tonic::Request<notify::CheckCrewsResourceRequest>) -> std::result::Result<tonic::Response<notify::CheckCrewsResourceResponse>, tonic::Status> {
        
        let report_crews_resource = request.into_inner();
        let common = self.common.upgrade().expect("upgrade common failed");
        let common = common.lock().expect("common lock failed");

        if let Some(resource) = &common.resources {
            

            
            //resource.lock().expect("roster lock failed").remove(crew);
        }
        
        return Ok(tonic::Response::new(notify::CheckCrewsResourceResponse {
            compiler_path: String::new(),
            winkits_includes_path: Vec::new(),
            msvc_includes_path: String::new(),
            msvc_version: String::new(),
            error_code: 0,
            error_message: "".to_string(),
        }));
    }
    
}
