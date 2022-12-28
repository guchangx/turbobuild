
extern crate axum;

pub struct NetworkRequestHandler {
    coordinator_addr: String,
    _workers_addr: Vec<String>,
}

impl Default for NetworkRequestHandler {
    fn default() -> Self {
        Self {
            coordinator_addr: "127.0.0.1:9302".to_string(),
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

async fn request_compile(axum::extract::Json(compile_input): axum::extract::Json<crate::compiler::compiler::CompileInput>, 
        working_parameters: crate::buildturbo::WorkingParameters, thread_pool: tokio::runtime::Handle) -> axum::extract::Json<serde_json::Value> {
    log::trace!("local request compile");
    let output = crate::compiler::compiler::request_compile(working_parameters, compile_input, &thread_pool).await;
    return axum::extract::Json(serde_json::json!(output));
}

async fn dist_request_compile(axum::extract::Json(compile_input): axum::extract::Json<crate::compiler::compiler::CompileInput>,
        working_parameters: crate::buildturbo::WorkingParameters, thread_pool: tokio::runtime::Handle) -> axum::extract::Json<serde_json::Value>
{
    println!("dist request compile");
    let output = crate::compiler::compiler::dist_request_compile(working_parameters, compile_input, &thread_pool).await;
    return axum::extract::Json(serde_json::json!(output));
}

async fn pre_sync_file(axum::extract::Json(pre_sync_file): axum::extract::Json<crate::compiler::compiler::SyncData>) -> axum::extract::Json<serde_json::Value> {
    let  exists_info = crate::syncfile::receiver::pre_sync_file(&pre_sync_file).await;
    return axum::extract::Json(serde_json::json!(exists_info));
}

async fn sync_file(mut multipart: axum::extract::multipart::Multipart) -> axum::extract::Json<serde_json::Value> {
    let exists_info = crate::syncfile::receiver::sync_file_to_local(&mut multipart).await;
    return axum::extract::Json(serde_json::json!(exists_info));
}

async fn init_network_request_router(working_params: crate::buildturbo::WorkingParameters, thread_pool: &tokio::runtime::Handle) {
    let pool = thread_pool.clone();
    let dist_pool = thread_pool.clone();
    let dist_working_params = working_params.clone();
    let router = axum::Router::new()
    .route("/", axum::routing::get(|| async {"Hi!"}))
    .route("/hello", axum::routing::get(get_hello_info))
    .route("/teamworker", axum::routing::get(get_teamworker_info))
    .route("/requestcompile", axum::routing::post(|args| {
                request_compile(args, working_params, pool)
            }
        ))
    .route("/dist/requestcompile", axum::routing::post(|args| {
                println!("into dist request compile");
                //request_compile(args, dist_working_params, dist_pool)
                dist_request_compile(args, dist_working_params, dist_pool)
            }
        ))
    .route("/dist/presyncfile", axum::routing::post(pre_sync_file))
    .route("/dist/syncfile", axum::routing::post(sync_file))
    .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 50));

    let network = NetworkRequestHandler::default();
    let addr = network.coordinator_addr.as_str().parse::<std::net::SocketAddr>().unwrap();

    match axum::Server::try_bind(&addr) {
        Ok(builder) => {
            let server = builder.serve(router.into_make_service());
            if let Err(err) = server.await {
                println!("start service error: {}", err);
            }
        },
        Err(error) => {
            println!("start service bing to a address {:?} failed. {:?}", addr.ip(), error);
        }
    }
}

impl NetworkRequestHandler {
    pub fn start(working_params: crate::buildturbo::WorkingParameters) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let handle = runtime.handle();
        runtime.block_on(async move {
            init_network_request_router(working_params, handle).await
        });
    } 
}
