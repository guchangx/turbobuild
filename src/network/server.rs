
extern crate axum;


lazy_static! {
    static ref WORKING_PARAMETERS: std::sync::Mutex<crate::buildturbo::WorkingParameters> = std::sync::Mutex::new(crate::buildturbo::WorkingParameters::default());
}

pub struct NetworkRequestHandler {
    commonder_addr: String,
    workers_addr: Vec<String>,

}

impl Default for NetworkRequestHandler {
    fn default() -> Self {
        Self {
            commonder_addr: "127.0.0.1:9302".to_string(),
            workers_addr: vec!["127.0.0.1:9302".to_string()],
        }
    }
}

async fn get_hello_info() -> axum::extract::Json<serde_json::Value>
{
    axum::extract::Json(serde_json::json!("hello, welcome use build turbo."))
}

async fn get_teamworker_info() -> axum::extract::Json<serde_json::Value>
{
    axum::extract::Json(serde_json::json!("get team worker info"))
}

async fn request_compile(axum::extract::Json(compile_input): axum::extract::Json<crate::compiler::compiler::CompileInput>) -> axum::extract::Json<serde_json::Value> {

    let parameters = WORKING_PARAMETERS.lock().unwrap().to_owned();
    let output = crate::compiler::compiler::request_compile(compile_input, parameters).await;

    return axum::extract::Json(serde_json::json!(output))
}

async fn init_network_request_router() {
    let router = axum::Router::new()
    .route("/", axum::routing::get(|| async {"Hi!"}))
    .route("/hello", axum::routing::get(get_hello_info))
    .route("/teamworker", axum::routing::get(get_teamworker_info))
    .route("/requestcompile", axum::routing::post(request_compile));

    let network = NetworkRequestHandler::default();
    let addr = network.commonder_addr.as_str().parse::<std::net::SocketAddr>().unwrap();
    
    let server = axum::Server::bind(&addr)
        .serve(router.into_make_service());
    
    if let Err(err) = server.await {
        println!("server error: {}", err);
    }  
}

impl NetworkRequestHandler {
    pub async fn start(working_params: crate::buildturbo::WorkingParameters) {
        init_network_request_router();
        WORKING_PARAMETERS.lock().unwrap().set(working_params);
    }
}
