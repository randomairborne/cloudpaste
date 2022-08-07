mod handlers;
use worker::*;

const SECONDS_IN_A_WEEK: u64 = 604_800;
const NAMESPACE: &str = "CLOUDPASTE";

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    log_request(&req);
    let router = Router::new();
    router
        .get("/", |_, _| Response::from_html(include_str!("index.html")))
        .get_async("/:id", handlers::template)
        .get_async("/raw/:id", handlers::raw)
        .post_async("/api/new", handlers::create)
        .post_async("/api/delete/:id/:token", handlers::delete)
        .get("/jbmono.woff", |_req, _ctx| {
            Response::from_bytes(include_bytes!("jbmono.woff").to_vec())
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
