use actix_easy_multipart::tempfile::Tempfile;
use actix_easy_multipart::text::Text;
use actix_easy_multipart::MultipartForm;
use actix_web::{post, App, HttpServer, Responder};

#[derive(MultipartForm)]
struct Upload {
    text: Text<String>,
    number: Text<i64>,
    file: Tempfile,
}

#[post("/")]
async fn route(form: MultipartForm<Upload>) -> impl Responder {
    let content_type = form
        .file
        .content_type
        .as_ref()
        .map(|m| m.as_ref())
        .unwrap_or("null");
    let file_name = form
        .file
        .file_name
        .as_ref()
        .map(|m| m.as_ref())
        .unwrap_or("null");

    format!(
        "Received:\ntext = {}\nnumber = {}\nfile = {} bytes, content-type = {}, filename = {}",
        &*form.text, &*form.number, form.file.size, content_type, file_name
    )
}

/// Test using `curl http://localhost:8080/ -F text="Hello World" -F number=23 -F file=@localfilename`
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(route))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
