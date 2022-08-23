mod gets;
mod posts;
use worker::*;

const SECONDS_IN_A_WEEK: u64 = 604_800;
const NAMESPACE: &str = "CLOUDPASTE";
const MAX_UPLOAD_BYTES: usize = 30_000_000;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located within {}, {}, {}, {}",
        Date::now().to_string(),
        req.path(),
        req.cf().city().unwrap_or_else(|| "(unknown)".into()),
        req.cf().region().unwrap_or_else(|| "(unknown)".into()),
        req.cf().country().unwrap_or_else(|| "(unknown)".into()),
        req.cf().continent().unwrap_or_else(|| "(unknown)".into()),
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    log_request(&req);
    let router = Router::new();
    router
        .get("/", |_, _| {
            Response::from_html(include_str!("html/index.html"))
        })
        .get("/about", |_, _| {
            Response::from_html(include_str!("html/about.html"))
        })
        .get("/main.css", gets::style)
        .get("/worker.js", gets::worker)
        .get_async("/:id", gets::template)
        .get_async("/raw/:id", gets::raw)
        .post_async("/api/delete/:id/:token", posts::delete)
        .post_async("/api/new", move |req, ctx| async {
            posts::new(req, ctx, false).await
        })
        .post_async("/api/new/nojs", move |req, ctx| async {
            posts::new(req, ctx, true).await
        })
        .run(req, env)
        .await
}

#[must_use]
fn randstr(length: usize) -> String {
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890"
        .chars()
        .collect();
    let mut result = String::with_capacity(length);
    let mut rng = rand::thread_rng();
    for _ in 0..length {
        result.push(
            *chars
                .get(rand::Rng::gen_range(&mut rng, 0..chars.len()))
                .unwrap_or(&'-'),
        );
    }
    result
}

fn error(err: &str, statuscode: u16, html: bool) -> Result<Response> {
    if html {
        let mut context = tera::Context::new();
        context.insert("error", err);
        let mut headers = Headers::new();
        headers.append("Content-Type", "text/html")?;
        if let Ok(resp_html) = tera::Tera::one_off(include_str!("html/error.html"), &context, true)
        {
            return Ok(Response::error(resp_html, statuscode)?.with_headers(headers));
        }
        Response::error(err, statuscode)
    } else {
        Response::error(err, statuscode)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PasteMetadata {
    revoke: String,
    language: String,
}
