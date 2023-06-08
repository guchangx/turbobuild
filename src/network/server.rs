
use crate::utils;
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
        working_parameters: crate::buildturbo::WorkingParameters, thread_pool: tokio::runtime::Handle, 
        grade: std::sync::Arc<std::sync::Mutex<utils::grade::LocalGrade>>) -> axum::extract::Json<serde_json::Value> {
    log::trace!("local request compile");
    let output = crate::compiler::compiler::request_compile(working_parameters, compile_input, &thread_pool, grade).await;
    let output = axum::extract::Json(serde_json::json!(output.0));

    return output;
}

async fn dist_request_compile(axum::extract::Json(compile_input): axum::extract::Json<crate::compiler::compiler::CompileInput>,
        working_parameters: crate::buildturbo::WorkingParameters, thread_pool: tokio::runtime::Handle,
        grade: std::sync::Arc<std::sync::Mutex<utils::grade::LocalGrade>>) -> axum::extract::Json<serde_json::Value>
{
    log::trace!("dist request compile");
    let output: (crate::compiler::compiler::CompileOutput, Option<Vec<crate::compiler::compiler::ProcessedResult>>) = crate::compiler::compiler::dist_request_compile(working_parameters, compile_input, &thread_pool, grade).await;
    return axum::extract::Json(serde_json::json!(output));
}

async fn remote_request_compile(multipart: axum::extract::multipart::Multipart, working_parameters: crate::buildturbo::WorkingParameters, 
                                thread_pool: tokio::runtime::Handle,
                                grade: std::sync::Arc<std::sync::Mutex<utils::grade::LocalGrade>>) 
                            -> axum::response::Response<axum::body::Full<axum::body::Bytes>> {
        
    let (output, results) = crate::compiler::compiler::remote_request_compile(multipart, working_parameters, &thread_pool, grade).await;

    let mut files: Vec<Vec<(std::ffi::OsString, usize)>> = Vec::new();
    let mut contents = Vec::<u8>::new();
    if let Some(results) = &results {
        for result in results {
            if let Some((path, content)) = &result.obj {
                files.push(vec![(path.to_owned(), content.len())]);
                contents.append(content.to_owned().as_mut());
            }
            if let Some((path, content)) = &result.pdb {
                files.push(vec![(path.to_owned(), content.len())]);
                contents.append(content.to_owned().as_mut());
            }
            if let Some((path, content)) = &result.idb {
                files.push(vec![(path.to_owned(), content.len())]);
                contents.append(content.to_owned().as_mut());
            }
        }
    }

    let bytes = axum::body::Bytes::from(contents);
    let response = axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "multipart/form-data")
        .header("compile-output", serde_json::to_string(&output).expect("serialize CompileOutput struct into json failed."))
        .header("compile-result-catalog", serde_json::to_string(&files).expect("serialize CompileResultCount struct into json failed."))
        .body(axum::body::Full::from(bytes))  
        .unwrap();
    return response;
}

async fn remote_request_compile_1(multipart: axum::extract::multipart::Multipart, working_parameters: crate::buildturbo::WorkingParameters, 
                                thread_pool: tokio::runtime::Handle,
                                grade: std::sync::Arc<std::sync::Mutex<utils::grade::LocalGrade>>) 
                            -> axum::response::Response<axum::body::Full<axum::body::Bytes>> {
        
    let (output, results) = crate::compiler::compiler::remote_request_compile(multipart, working_parameters, &thread_pool, grade).await;

    let mut files: Vec<Vec<(std::ffi::OsString, usize)>> = Vec::new();
    let mut contents = Vec::<u8>::new();
    if let Some(results) = &results {
        for result in results {
            if let Some((path, content)) = &result.obj {
                files.push(vec![(path.to_owned(), content.len())]);
                contents.append(content.to_owned().as_mut());
            }
            if let Some((path, content)) = &result.pdb {
                files.push(vec![(path.to_owned(), content.len())]);
                contents.append(content.to_owned().as_mut());
            }
            if let Some((path, content)) = &result.idb {
                files.push(vec![(path.to_owned(), content.len())]);
                contents.append(content.to_owned().as_mut());
            }
        }
    }

    let bytes = axum::body::Bytes::from(contents);
    let response = axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "multipart/form-data")
        .header("compile-output", serde_json::to_string(&output).expect("serialize CompileOutput struct into json failed."))
        .header("compile-result-catalog", serde_json::to_string(&files).expect("serialize CompileResultCount struct into json failed."))
        .body(axum::body::Full::from(bytes))  
        .unwrap();
    return response;
}

async fn pre_sync_file(axum::extract::Json(pre_sync_file): axum::extract::Json<crate::compiler::compiler::SyncData>) -> axum::extract::Json<serde_json::Value> {
    let now = std::time::Instant::now();
    let  exists_info = crate::syncfile::receiver::pre_sync_file(&pre_sync_file).await;
    let response = axum::extract::Json(serde_json::json!(exists_info));
    log::trace!("pre sync file elapsed: {:?}", now.elapsed());
    return response;
}

async fn sync_file(mut multipart: axum::extract::multipart::Multipart) -> axum::extract::Json<serde_json::Value> {
    let exists_info = crate::syncfile::receiver::sync_file_to_local(&mut multipart).await;
    return axum::extract::Json(serde_json::json!(exists_info));
}

async fn init_network_request_router(working_params: crate::buildturbo::WorkingParameters, thread_pool: &tokio::runtime::Handle) {
    let pool = thread_pool.clone();
    let dist_pool = thread_pool.clone();
    let remote_compile_pool = thread_pool.clone();
    let remote_compile_pool_1 = thread_pool.clone();

    let dist_working_params = working_params.clone();
    let remote_compile_working_params = working_params.clone();
    let remote_compile_working_params_1 = working_params.clone();
    
    let grade = crate::utils::grade::LocalGrade::init_grade();
    let grade = std::sync::Arc::new(std::sync::Mutex::new(grade));
    crate::utils::grade::calculate_machine_residual_performance(grade.clone());
    let grade_clone = grade.clone();
    let remote_compile_grade_clone = grade.clone();
    let remote_compile_grade_clone_1 = grade.clone();

    let router = axum::Router::new()
    .route("/", axum::routing::get(|| async {"Hi!"}))
    .route("/hello", axum::routing::get(get_hello_info))
    .route("/teamworker", axum::routing::get(get_teamworker_info))
    .route("/requestcompile", axum::routing::post(move |args| {
                request_compile(args, working_params, pool, grade.clone())
            }
        ))
    .route("/dist/requestcompile/sourcefile", axum::routing::post(move |args| {
                dist_request_compile(args, dist_working_params, dist_pool, grade_clone.clone())
            }
        ))
    .route("/dist/requestcompile/precompiled", axum::routing::post(move |args| {
            remote_request_compile(args, remote_compile_working_params, remote_compile_pool, remote_compile_grade_clone.clone())
        }))
    .route("/dist/requestcompile/precompiled_1", axum::routing::post(move |args| {
            remote_request_compile_1(args, remote_compile_working_params_1, remote_compile_pool_1, remote_compile_grade_clone_1.clone())
        }))
    .route("/dist/presyncfile", axum::routing::post(pre_sync_file))
    .route("/dist/syncfile", axum::routing::post(sync_file))
    .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 50));

    let network = NetworkRequestHandler::default();
    let addr = network.coordinator_addr.as_str().parse::<std::net::SocketAddr>().unwrap();

    match axum::Server::try_bind(&addr) {
        Ok(builder) => {
            let server = builder.serve(router.into_make_service());
            if let Err(err) = server.await {
                println!("start service error: {:?}", err);
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
