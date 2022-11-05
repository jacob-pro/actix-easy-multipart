use actix_easy_multipart::text::Text;
use actix_easy_multipart::MultipartForm;
use actix_web::{post, App, HttpServer, Responder};

#[derive(MultipartForm)]
struct Upload {
    text: Text<String>,
}

#[post("/")]
async fn route(form: MultipartForm<Upload>) -> impl Responder {
    format!("Received text={}", &*form.text)
}

/// Test using `curl http://localhost:8080/ -v -F text=HelloWorld`
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(route))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
