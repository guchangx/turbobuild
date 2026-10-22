
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
                            let mut task: Option<crate::roster::crews::Task> = None;
                            if let Some(addr) = addr {
                                let common = common.lock().expect("common lock failed");
                                match serde_json::from_str::<crate::roster::crews::CrewRegister>(&notification.message) {
                                    Ok(mut crew) => {
                                        crew.addr = addr.ip().to_string();
                                        log::info!("Crew unregistered: {}@{} (addr={})", crew.username, crew.devicename, crew.addr);
                                        if let Some(roster) = &common.constitutions {
                                            roster.lock().expect("roster lock failed").remove(crew.clone());
                                        }
                                        if let Some(roster) = &common.resources {
                                            let mut resourcelist = roster.lock().expect("roster lock failed");
                                            resourcelist.remove(&crew.addr, &crew.username, &crew.devicename);
                                        }
                                        if let Some(tasks) = &common.tasks {
                                            let mut tasks = tasks.lock().unwrap();
                                            task = tasks.remove(&crew.addr, &crew.username, &crew.devicename);
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("Failed to parse unregister payload from {:?}: {}", addr, err);
                                    }
                                }
                            }
                            
                            if let Some(task) = task {
                                let message = serde_json::to_string(&vec![task]).unwrap();
                                let reply = notify::NotifyResponse {
                                    r#type: notify::Type::Unregister as i32,
                                    message,
                                    sequence: notification.sequence,
                                    error_code: 0,
                                    error_message: "unregister success".to_string(),
                                };
                                self_.broadcast(reply).await;
                            }
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
                        if let Some(source) = err.source() {
                            if let Some(hyper_source) = source.source() {
                                if let Some(inner_source) = hyper_source.source() {
                                    if let Some(err) = inner_source.downcast_ref::<hyper::Error>() {
                                        if let Some(_source) = err.source() {
                                            
                                            let mut task: Option<crate::roster::crews::Task> = None;
                                            if let Some(addr) = addr {
                                                let ip = addr.ip().to_string();
                                                let common = common.lock().expect("common lock failed");
                                                if let Some(roster) = &common.resources {
                                                    let mut resourcelist = roster.lock().expect("roster lock failed");
                                                    resourcelist.remove(&ip, "", "");
                                                }

                                                task = if let Some(tasks) = &common.tasks {
                                                    let mut tasks = tasks.lock().unwrap();
                                                    tasks.remove(&ip, "", "")
                                                }
                                                else {
                                                    None
                                                }
                                            }

                                            if let Some(task) = task {
                                                let message = serde_json::to_string(&vec![task]).unwrap();
                                                let reply = notify::NotifyResponse {
                                                    r#type: notify::Type::Unregister as i32,
                                                    message,
                                                    sequence: 0,
                                                    error_code: 0,
                                                    error_message: "client disconnected".to_string(),
                                                };
                                                self_.broadcast(reply).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        log::debug!("Client stream error from {:?}: {}", addr, err);
                        break;
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
                manage.lock().expect("roster lock failed").remove(&resource.addr, &resource.username, &resource.devicename);
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::notify::communicate_server::Communicate;

    #[test]
    fn test_notification_receiver_new() {
        let receiver = NotificationReceiver::new(std::sync::Weak::new());
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let broadcaster = receiver.broadcaster.lock().await;
            assert!(broadcaster.is_empty());
        });
    }

    #[test]
    fn test_notification_receiver_broadcast() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let receiver = NotificationReceiver::new(std::sync::Weak::new());
            let (tx, mut rx) = tokio::sync::mpsc::channel(16);

            receiver.broadcaster.lock().await.push(tx);

            let msg = notify::NotifyResponse {
                r#type: notify::Type::Register as i32,
                message: "broadcast payload".to_string(),
                sequence: 42,
                error_code: 0,
                error_message: "ok".to_string(),
            };

            receiver.broadcast(msg.clone()).await;

            let received = rx.recv().await.expect("receive broadcast message");
            let resp = received.expect("valid notify response");
            assert_eq!(resp.r#type, notify::Type::Register as i32);
            assert_eq!(resp.message, "broadcast payload");
            assert_eq!(resp.sequence, 42);

            // Now drop rx to close the channel
            drop(rx);

            // Broadcasting again should prune the closed sender
            receiver.broadcast(msg).await;
            let broadcaster = receiver.broadcaster.lock().await;
            assert!(broadcaster.is_empty());
        });
    }

    #[test]
    fn test_notification_receiver_report_crews_resource() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let common = std::sync::Arc::new(std::sync::Mutex::new(crate::common::Common::new()));
            let weak_common = std::sync::Arc::downgrade(&common);

            let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
            let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
            common.lock().unwrap().resources = Some(arc_resources.clone());

            let receiver = NotificationReceiver::new(weak_common);

            let req = tonic::Request::new(notify::ReportCrewsResourceRequest {
                resources: vec![notify::CrewsResource {
                    username: "dev_user".to_string(),
                    aliasname: "build_machine".to_string(),
                    devicename: "pc_host".to_string(),
                    addr: "10.0.0.2".to_string(),
                    tool_versions: vec![
                        notify::ToolVersion {
                            version: "19.38.32919".to_string(),
                            host: 4, // x64
                            target: 4, // x64
                        }
                    ],
                }],
            });

            let resp = receiver.report_crews_resource(req).await.expect("report_crews_resource ok");
            assert_eq!(resp.get_ref().error_code, 0);

            // Verify the crew resource was added to common.resources
            let crews = arc_resources.lock().unwrap().check("", "", "");
            assert_eq!(crews.len(), 1);
            assert_eq!(crews[0].username, "dev_user");
            assert_eq!(crews[0].aliasname, "build_machine");
            assert_eq!(crews[0].devicename, "pc_host");
            assert_eq!(crews[0].addr, "10.0.0.2");
            assert_eq!(crews[0].compiler_versions.len(), 1);
            assert_eq!(crews[0].compiler_versions[0].version, "19.38.32919");
            assert_eq!(crews[0].compiler_versions[0].host, crate::roster::crews::Arch::x64);
        });
    }

    #[test]
    fn test_notification_receiver_cancel_crews_resource() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let common = std::sync::Arc::new(std::sync::Mutex::new(crate::common::Common::new()));
            let weak_common = std::sync::Arc::downgrade(&common);

            let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
            let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
            common.lock().unwrap().resources = Some(arc_resources.clone());

            let receiver = NotificationReceiver::new(weak_common);

            // First report a resource
            let report_req = tonic::Request::new(notify::ReportCrewsResourceRequest {
                resources: vec![notify::CrewsResource {
                    username: "worker_x".to_string(),
                    aliasname: "build_box_x".to_string(),
                    devicename: "node_x".to_string(),
                    addr: "10.0.0.99".to_string(),
                    tool_versions: vec![],
                }],
            });
            let _ = receiver.report_crews_resource(report_req).await.unwrap();
            assert_eq!(arc_resources.lock().unwrap().check("", "", "").len(), 1);

            // Now cancel this resource
            let cancel_req = tonic::Request::new(notify::ReportCrewsResourceRequest {
                resources: vec![notify::CrewsResource {
                    username: "worker_x".to_string(),
                    aliasname: "build_box_x".to_string(),
                    devicename: "node_x".to_string(),
                    addr: "10.0.0.99".to_string(),
                    tool_versions: vec![],
                }],
            });
            let resp = receiver.cancel_crews_resource(cancel_req).await.unwrap();
            assert_eq!(resp.get_ref().error_code, 0);

            // Resource should be removed
            let remaining = arc_resources.lock().unwrap().check("", "", "");
            assert!(remaining.is_empty());
        });
    }

    #[test]
    fn test_notification_receiver_check_crews_resource() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let common = std::sync::Arc::new(std::sync::Mutex::new(crate::common::Common::new()));
            let weak_common = std::sync::Arc::downgrade(&common);

            let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
            let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
            common.lock().unwrap().resources = Some(arc_resources.clone());

            let receiver = NotificationReceiver::new(weak_common);

            // Report a resource
            let report_req = tonic::Request::new(notify::ReportCrewsResourceRequest {
                resources: vec![notify::CrewsResource {
                    username: "dev1".to_string(),
                    aliasname: "alias1".to_string(),
                    devicename: "pc1".to_string(),
                    addr: "192.168.1.100".to_string(),
                    tool_versions: vec![
                        notify::ToolVersion {
                            version: "19.38".to_string(),
                            host: 4,
                            target: 4,
                        }
                    ],
                }],
            });
            let _ = receiver.report_crews_resource(report_req).await.unwrap();

            // Query check_crews_resource with empty filter (returns all)
            let check_req = tonic::Request::new(notify::CheckCrewsResourceRequest {
                username: "".to_string(),
                aliasname: "".to_string(),
                addr: "".to_string(),
            });
            let resp = receiver.check_crews_resource(check_req).await.unwrap();
            let list = resp.into_inner().resources;
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].username, "dev1");
            assert_eq!(list[0].addr, "192.168.1.100");
            assert_eq!(list[0].tool_versions.len(), 1);
            assert_eq!(list[0].tool_versions[0].version, "19.38");
        });
    }

    #[test]
    fn test_notification_receiver_notify_stream_register_and_unregister() {
        use tokio_stream::StreamExt;
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let common = std::sync::Arc::new(std::sync::Mutex::new(crate::common::Common::new()));
            let weak_common = std::sync::Arc::downgrade(&common);

            let tasks = crate::roster::crews::TaskManager::new();
            let arc_tasks = std::sync::Arc::new(std::sync::Mutex::new(tasks));
            common.lock().unwrap().tasks = Some(arc_tasks.clone());

            let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
            let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
            common.lock().unwrap().resources = Some(arc_resources.clone());

            let constitutions = crate::roster::crews::ConstitutionList::new(weak_common.clone());
            let arc_constitutions = std::sync::Arc::new(std::sync::Mutex::new(constitutions));
            common.lock().unwrap().constitutions = Some(arc_constitutions.clone());

            let receiver = NotificationReceiver::new(weak_common);
            let server = notify::communicate_server::CommunicateServer::new(receiver);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let local_addr = listener.local_addr().unwrap();

            tokio::spawn(async move {
                let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                let _ = tonic::transport::Server::builder()
                    .add_service(server)
                    .serve_with_incoming(incoming)
                    .await;
            });

            let mut client = notify::communicate_client::CommunicateClient::connect(format!("http://{}", local_addr))
                .await
                .expect("connect to local captain server");

            let (req_tx, req_rx) = tokio::sync::mpsc::channel(16);
            let in_stream = tokio_stream::wrappers::ReceiverStream::new(req_rx);

            let resp = client.notify(in_stream).await.expect("notify stream established");
            let mut out_stream = resp.into_inner();

            // 1. Send Register request
            let register_crew = crate::roster::crews::CrewRegister {
                username: "stream_user".to_string(),
                devicename: "stream_host".to_string(),
                addr: "127.0.0.1".to_string(),
                passcode: "token".to_string(),
                core: 4,
                frequency: vec![2400],
                memory: 16.0,
            };
            let reg_msg = serde_json::to_string(&register_crew).unwrap();
            req_tx.send(notify::NotifyRequest {
                r#type: notify::Type::Register as i32,
                message: reg_msg,
                sequence: 1,
            }).await.unwrap();

            // Receive broadcast/response for Register
            let reg_resp = out_stream.next().await.unwrap().unwrap();
            assert_eq!(reg_resp.r#type, notify::Type::Register as i32);
            assert_eq!(reg_resp.sequence, 1);

            // Verify task was added
            let current_tasks = arc_tasks.lock().unwrap().check("", "", "");
            assert_eq!(current_tasks.len(), 1);
            assert_eq!(current_tasks[0].username, "stream_user");
            assert_eq!(current_tasks[0].addr, "127.0.0.1");

            // 2. Send Unregister request
            let unreg_msg = serde_json::to_string(&register_crew).unwrap();
            req_tx.send(notify::NotifyRequest {
                r#type: notify::Type::Unregister as i32,
                message: unreg_msg,
                sequence: 2,
            }).await.unwrap();

            // Receive response for Unregister
            let unreg_resp = out_stream.next().await.unwrap().unwrap();
            assert_eq!(unreg_resp.r#type, notify::Type::Unregister as i32);
            assert_eq!(unreg_resp.sequence, 2);

            let unreg_tasks: Vec<crate::roster::crews::Task> = serde_json::from_str(&unreg_resp.message).unwrap();
            assert_eq!(unreg_tasks.len(), 1);
            assert_eq!(unreg_tasks[0].username, "stream_user");
            assert_eq!(unreg_tasks[0].devicename, "stream_host");
            assert!(!unreg_tasks[0].online);

            // Verify task was marked offline
            let remaining_tasks = arc_tasks.lock().unwrap().check("", "", "");
            assert_eq!(remaining_tasks.len(), 1);
            assert!(!remaining_tasks[0].online);
            assert_eq!(arc_tasks.lock().unwrap().alls().len(), 1);
            assert!(!arc_tasks.lock().unwrap().alls()[0].online);
        });
    }

    #[test]
    fn test_notification_receiver_notify_stream_checkresource_and_keepalive() {
        use tokio_stream::StreamExt;
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let common = std::sync::Arc::new(std::sync::Mutex::new(crate::common::Common::new()));
            let weak_common = std::sync::Arc::downgrade(&common);

            let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
            let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
            common.lock().unwrap().resources = Some(arc_resources.clone());

            let constitutions = crate::roster::crews::ConstitutionList::new(weak_common.clone());
            let arc_constitutions = std::sync::Arc::new(std::sync::Mutex::new(constitutions));
            common.lock().unwrap().constitutions = Some(arc_constitutions.clone());

            let receiver = NotificationReceiver::new(weak_common);
            let server = notify::communicate_server::CommunicateServer::new(receiver);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let local_addr = listener.local_addr().unwrap();

            tokio::spawn(async move {
                let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                let _ = tonic::transport::Server::builder()
                    .add_service(server)
                    .serve_with_incoming(incoming)
                    .await;
            });

            let mut client = notify::communicate_client::CommunicateClient::connect(format!("http://{}", local_addr))
                .await
                .expect("connect to local captain server");

            let (req_tx, req_rx) = tokio::sync::mpsc::channel(16);
            let in_stream = tokio_stream::wrappers::ReceiverStream::new(req_rx);

            let resp = client.notify(in_stream).await.expect("notify stream established");
            let mut out_stream = resp.into_inner();

            // 1. Send Checkresource
            let resource = crate::roster::crews::CrewResource {
                username: "chk_user".to_string(),
                aliasname: "chk_alias".to_string(),
                devicename: "chk_dev".to_string(),
                addr: "".to_string(), // will be populated from remote addr
                compiler_versions: vec![],
            };
            let res_json = serde_json::to_string(&resource).unwrap();
            req_tx.send(notify::NotifyRequest {
                r#type: notify::Type::Checkresource as i32,
                message: res_json,
                sequence: 10,
            }).await.unwrap();

            let res_resp = out_stream.next().await.unwrap().unwrap();
            assert_eq!(res_resp.r#type, notify::Type::Checkresource as i32);
            assert_eq!(res_resp.sequence, 10);
            assert_eq!(res_resp.error_code, 0);

            // Verify resource list updated with remote IP
            let list = arc_resources.lock().unwrap().check("", "", "");
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].username, "chk_user");
            assert_eq!(list[0].addr, "127.0.0.1");

            // 2. Send Keepalive
            let constitution = crate::roster::crews::CrewConstitution {
                username: "chk_user".to_string(),
                devicename: "chk_dev".to_string(),
                aliasname: "chk_alias".to_string(),
                role: 0,
                status: 1,
                action: 0,
                addr: "".to_string(),
                physical_cores: 8,
                virtual_cores: 16,
                cpu_usage: 10.0,
                cpu_frequency: vec![3000],
                memory_total: 16.0,
                memory_usage: 4.0,
                os_version: "Win11".to_string(),
                description: "test".to_string(),
            };
            let const_json = serde_json::to_string(&constitution).unwrap();
            req_tx.send(notify::NotifyRequest {
                r#type: notify::Type::Keepalive as i32,
                message: const_json,
                sequence: 11,
            }).await.unwrap();

            let keepalive_resp = out_stream.next().await.unwrap().unwrap();
            assert_eq!(keepalive_resp.r#type, notify::Type::Keepalive as i32);
            assert_eq!(keepalive_resp.sequence, 11);
            assert_eq!(keepalive_resp.error_code, 0);
        });
    }
}


