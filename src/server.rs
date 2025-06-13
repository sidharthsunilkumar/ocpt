use axum::{
    routing::{get, post},
    Router,
};
use serde_json::Value;
use tokio::fs;

// GET / — serves the content of dfg.json as JSON
async fn hello() -> Json<Value> {
    let file_content = fs::read_to_string("dfg.json")
        .await
        .expect("Failed to read dfg.json");
    let json: Value = serde_json::from_str(&file_content)
        .expect("Failed to parse JSON from dfg.json");

    Json(json)
}

// Simple POST endpoint - prints body to console
async fn print_body(body: String) -> String {
    println!("Received body: {}", body);
    "Body printed to console".to_string()
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(hello))
        .route("/print", post(print_body));

    println!("Server running on http://localhost:1080");
    println!("GET  / - Hello World");
    println!("POST /print - Prints body to console");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}