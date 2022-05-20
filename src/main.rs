extern crate axum;

async fn get_hello_info() -> &'static str
{
    "hello, welcome use build turbo."
}

async fn get_teamworker_info() -> &'static str
{
    "get team worker info"
}

async fn do_build() -> &'static str
{
    "do build"
}

async fn test_json() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({"data": 123}))
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {

    let router = axum::Router::new()
            .route("/hello", axum::routing::get(get_hello_info))
            .route("/teamworker", axum::routing::get(get_teamworker_info))
            .route("/dobuild", axum::routing::post(do_build))
            .route("/testjson", axum::routing::get(test_json).post(test_json));

    let addr = &"127.0.0.1:9302".parse().unwrap();
    let server = axum::Server::bind(addr)
        .serve(router.into_make_service());
        
    if let Err(err) = server.await {
        println!("server error: {}", err);
    }  
}

