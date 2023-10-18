use actix_files::Files;
use actix_web::{
    get,
    http::header::{Header, HeaderValue},
    post,
    web::{self, post, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use dotenv::dotenv;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, FromRow, Pool, Postgres};
use std::env;
use validator::Validate;

use std::io::Write;
use llm::{Model, ModelParameters, TokenizerSource, InferenceParameters};


#[derive(Debug)]
pub struct AppState {
    db: Pool<Postgres>,
    secret: String,
    pub token: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TodoRequest {
    pub todo: String,
    pub date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexData {
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct ResponsiveTableData {
    pub table_headers: Vec<String>,
    pub table_rows: Vec<ResponsiveTableRow>,
}
#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct ResponsiveTableRow {
    pub tds: Vec<String>,
}

#[get("/")]
async fn index(
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
    state: Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let headers = req.headers();
    for (pos, e) in headers.iter().enumerate() {
        println!("Element at position {}: {:?}", pos, e);
    }
    let data = json!({
        "header": "Login Form",
    });
    let body = hb.render("ui_main", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct PromptRequest {
    pub prompt: String,
}

#[get("/prompt")]
    async fn llm_prompt(hb: web::Data<Handlebars<'_>>, body: web::Form<PromptRequest>,) -> impl Responder {
        // load a GGML model from disk
        let llama = llm::load::<llm::models::Llama>(
        std::path::Path::new("/path/to/model"),
        // llm::ModelParameters
        llm::TokenizerSource::Embedded,
        ModelParameters::default(),
        // load progress callback
        llm::load_progress_callback_stdout
        )
        .unwrap_or_else(|err| panic!("Failed to load model: {err}"));

    // use the model to generate text from a prompt
    let mut session = llama.start_session(Default::default());
    let res = session.infer::<std::convert::Infallible>(
        // model to use for text generation
        &llama,
        // randomness provider
        &mut rand::thread_rng(),
        // the prompt to use for text generation, as well as other
        // inference parameters
        &llm::InferenceRequest {
            prompt: llm::Prompt::Text(&body.prompt),
            parameters: &InferenceParameters::default(),
            play_back_previous_tokens: true,
            /// The maximum number of tokens to generate.
            maximum_token_count: Some(100),
        },
        // llm::OutputRequest
        &mut Default::default(),
        // output callback
        |t| {
            print!("{:?}", t);
            std::io::stdout().flush().unwrap();

            Ok(t)
        }
    );
    let body = hb.render("llm_reponse", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[get("/list")]
async fn list_api(hb: web::Data<Handlebars<'_>>) -> impl Responder {
    let data = json!({
        "name": "Lists",
        "title": "View Records",
    });
    let body = hb.render("list-api", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct ValidationError {
    error: String,
}
#[derive(Debug, FromRow, Validate, Serialize, Deserialize)]
pub struct ValidatedUser {
    username: String,
    email: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }
    env_logger::init();
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").unwrap_or("NoURL".to_string());
    // let database_url = env!("DATABASE_URL");
    // let secret = std::env::var("JWT_SECRET").unwrap_or(env!("JWT_SECRET").to_owned());
    let secret = "temp_secret";
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    let mut handlebars = Handlebars::new();

    handlebars
        .register_templates_directory(".hbs", "./templates")
        .unwrap();

    // handlebars.register_helper("to_title_case", Box::new(to_title_case));

    handlebars.set_dev_mode(true);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db: pool.clone(),
                secret: secret.to_string(),
                token: "".to_string().clone(),
            }))
            .app_data(web::Data::new(handlebars.clone()))
            .service(index)
            // .service(responsive_table)
            .service(
                Files::new("/", "./static")
                    .prefer_utf8(true)
                    .use_last_modified(true),
            )
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
