
extern crate axum;

pub struct NetworkRequestHandler {
    commonder_addr: String,
    _workers_addr: Vec<String>,

}

impl Default for NetworkRequestHandler {
    fn default() -> Self {
        Self {
            commonder_addr: "127.0.0.1:9302".to_string(),
            _workers_addr: vec!["127.0.0.1:9302".to_string()],
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

//async fn request_compile(axum::extract::Json(compile_input): axum::extract::Json<crate::compiler::compiler::CompileInput>, 
//        working_parameters: crate::buildturbo::WorkingParameters, pool: tokio::runtime::Handle)

async fn request_compile(axum::extract::Json(compile_input): axum::extract::Json<crate::compiler::compiler::CompileInput>, 
        working_parameters: crate::buildturbo::WorkingParameters, pool: tokio::runtime::Handle) -> axum::extract::Json<serde_json::Value> {

    let output = crate::compiler::compiler::request_compile(compile_input, working_parameters, &pool).await;
    return axum::extract::Json(serde_json::json!(output));

}
//async fn init_network_request_router(working_params: crate::buildturbo::WorkingParameters, thread_pool: tokio::runtime::Handle) 
async fn init_network_request_router(working_params: crate::buildturbo::WorkingParameters, pool: &tokio::runtime::Handle) {
    let pool = pool.clone();
    let router = axum::Router::new()
    .route("/", axum::routing::get(|| async {"Hi!"}))
    .route("/hello", axum::routing::get(get_hello_info))
    .route("/teamworker", axum::routing::get(get_teamworker_info))
    .route("/requestcompile", axum::routing::post( |args| {
                request_compile(args, working_params, pool)
            }
        )
    );

    let network = NetworkRequestHandler::default();
    let addr = network.commonder_addr.as_str().parse::<std::net::SocketAddr>().unwrap();

    let server = axum::Server::bind(&addr)
        .serve(router.into_make_service());

    if let Err(err) = server.await {
        println!("start service error: {}", err);
    }
}

impl NetworkRequestHandler {
    pub fn start(working_params: crate::buildturbo::WorkingParameters) {
    
        // working_params.runtime.clone().block_on (
        //     async move {
        //         init_network_request_router(working_params).await
        //     }
        // );

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let handle = runtime.handle();

        runtime.block_on(async move {
            init_network_request_router(working_params, handle).await
         });

    }
}
