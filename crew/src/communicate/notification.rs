
#[allow(non_camel_case_types)]
pub mod notify {
    include!("../../proto/notify.rs");
}

#[derive(Default)] 
pub struct NotificationSender {}

pub enum NotificationType {
    Resource(String),
    Constitution(String),
}

impl NotificationSender {
    pub async fn register(&self, mut receiver: tokio::sync::mpsc::Receiver<NotificationType>) -> Result<String, String> {
        use tokio_stream::StreamExt;

        let mut client = notify::communicate_client::CommunicateClient::connect("http://localhost:50051").await.unwrap();

        let (tx, rx) = tokio::sync::mpsc::channel(128); 
        let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

        match client.notify(request_stream).await {
            Ok(response) => {
                
                let mut response_stream = response.into_inner();
                tokio::spawn(async move {
                    while let Some(stream) = response_stream.next().await {
                        match stream {
                            Ok(response) => {
                                println!("notify response: {}", response.message);
                            }
                            Err(err) => {
                                eprintln!("notify response: {:?}", err);
                            }
                        }
                    }
                });
            },
            Err(status) => {
                return Err(status.message().to_string());
            }
        }
       
        let request = notify::NotifyRequest {
            r#type: notify::Type::Register as i32,
            name: "register".to_string(),
        };
    
        if let Err(err) = tx.send(request).await {
            eprintln!("notify register error: {:?}", err);
        };

        while let Some(notification_type) = receiver.recv().await {
            match  notification_type {
                NotificationType::Resource(message) => {
                
                },
                NotificationType::Constitution(message) => {
                
                    let request = notify::NotifyRequest {
                        r#type: notify::Type::Keepalive as i32,
                        name: message,
                    };
                    
                    if let Err(err) = tx.send(request).await {
                        eprintln!("notify constitution error: {:?}", err);
                    };
                }
            }
        }

        return Ok("OK".to_string());
    }

    pub async fn report_resource(resources: Vec<notify::CrewsResource>) {
    
        let mut client = notify::communicate_client::CommunicateClient::connect("http://localhost:50051").await.unwrap();
       
       let request = notify::ReportCrewsResourceRequest {
            resources
       };
       
        match client.report_crews_resource(request).await {
            Ok(response) => {
                
                let response = response.into_inner();
                println!("response {:?}", response);
            },
            Err(err) => {
                println!("report crew resource. {:?}", err);
            }
        }
    }
 }