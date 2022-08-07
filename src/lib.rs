use worker::*;

const SECONDS_IN_A_WEEK: u64 = 604_800;
const NAMESPACE: &str = "CLOUDPASTE";

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let router = Router::new();
    router
        .get("/", |_, _| Response::from_html(include_str!("index.html")))
        .get("/:paste", |_, _| {
            Response::from_html(include_str!("paste.html"))
        })
        .get_async("/api/:id", |_req, ctx| async move {
            if let Ok(kv) = ctx.kv(NAMESPACE) {
                let key = match ctx.param("id") {
                    Some(val) => val,
                    None => return Response::error("No paste ID!", 400),
                };
                let maybe_value = match kv.get(key).text().await {
                    Ok(val) => val,
                    Err(e) => return Response::error(format!("KV Error: {}", e), 500),
                };
                if let Some(value) = maybe_value {
                    return Response::ok(value);
                }
                return Response::error("Paste Not Found!", 404);
            }
            Response::error("Account Misconfigured, no CLOUDPASTE kv found", 500)
        })
        .post_async("/api/new", |mut req, ctx| async move {
            if let Ok(kv) = ctx.kv(NAMESPACE) {
                let id = randstr(10);
                let data = match req.text().await {
                    Ok(val) => val,
                    Err(_) => return Response::error("Failed to get data from POST request", 400),
                };
                let put = match kv.put(&id, data) {
                    Ok(val) => val,
                    Err(e) => return Response::error(format!("KV error: {e}"), 500),
                };
                if let Err(e) = put.expiration_ttl(SECONDS_IN_A_WEEK).execute().await {
                    return Response::error(format!("Failed to insert into KV: {e}"), 500);
                }
                return Response::ok(format!("/{id}"));
            }
            Response::error("Account Misconfigured, no CLOUDPASTE kv found", 500)
        })
        .get(
            "/jbmono.woff",
            |_req, _ctx| Response::from_bytes(include_bytes!("jbmono.woff").to_vec()),
        )
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
