
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
    pub async fn register(&self, mut receiver: tokio::sync::mpsc::Receiver<String>) -> Result<String, String> {
      use tokio_stream::StreamExt;
        
       let mut client = notify::communicate_client::CommunicateClient::connect("http://localhost:50051").await.unwrap();
       
       let (tx, rx) = tokio::sync::mpsc::channel(128); 
       let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
       
       match client.notify(request_stream).await {
           Ok(response) => {
               
               let mut response_stream = response.into_inner();
               tokio::spawn(async move {
                    while let Some(message) = response_stream.next().await {
                        match message {
                            Ok(message) => {
                                println!("notify response: {}", message.message);
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

       while let Some(message) = receiver.recv().await {
           
       }

       let request = notify::NotifyRequest {
            r#type: notify::Type::Register as i32,
            name: "name".to_string(),
        };
        
        if let Err(err) = tx.send(request).await {
            eprintln!("notify register error: {:?}", err);
        };    

        loop {
            let request = notify::NotifyRequest {
                r#type: notify::Type::Keepalive as i32,
                name: "".to_string(),
            };
            
            if let Err(err) = tx.send(request).await {
                eprintln!("notify keepalive error: {:?}", err);
                break;
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;            
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