use worker::*;

use crate::{NAMESPACE, SECONDS_IN_A_WEEK};

pub async fn template(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(kv) = ctx.kv(NAMESPACE) {
        let key = match ctx.param("id") {
            Some(val) => val,
            None => return error("No paste ID!", 400),
        };
        let maybe_value = match kv.get(key).text().await {
            Ok(val) => val,
            Err(e) => return error(&format!("KV Error: {}", e), 500),
        };

        if let Some(value) = maybe_value {
            let mut context = tera::Context::new();
            context.insert("id", key);
            context.insert("content", &value);
            if let Ok(page) = tera::Tera::one_off(include_str!("paste.html"), &context, true) {
                return Response::from_html(page);
            }

            return error(
                "Templating failed! (this is a bug, github.com/randomairborne/cloudpaste)",
                500,
            );
        }

        return error("Paste Not Found!", 404);
    }
    Response::error("Account Misconfigured, no CLOUDPASTE kv found", 500)
}

pub async fn raw(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
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
}

pub async fn create(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(kv) = ctx.kv(NAMESPACE) {
        let id = crate::randstr(10);
        let revoke = crate::randstr(64);
        let data = match req.text().await {
            Ok(val) => val,
            Err(_) => return Response::error("Failed to get data from POST request", 400),
        };
        if data.len() > 20_000_000 {
            return Response::error("Oops, too long! Pastes must be less then 20MB", 400);
        }
        if data.is_empty() {
            return Response::error("Pastes must contain at least one character!", 400);
        }
        let put = match kv.put(&id, data) {
            Ok(val) => val,
            Err(e) => return Response::error(format!("KV error: {e}"), 500),
        };
        let meta = match put.metadata(&revoke) {
            Ok(val) => val,
            Err(e) => return Response::error(format!("Failed to serialize metadata: {e}"), 500),
        };
        if let Err(e) = meta.expiration_ttl(SECONDS_IN_A_WEEK).execute().await {
            return Response::error(format!("Failed to insert into KV: {e}"), 500);
        }
        return Response::from_json(&serde_json::json!({"id": id, "revoke": revoke}));
    }
    Response::error("Account Misconfigured, no CLOUDPASTE kv found", 500)
}

pub async fn delete(mut _req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(kv) = ctx.kv(NAMESPACE) {
        let id = match ctx.param("id") {
            Some(val) => val,
            None => return Response::error("No paste ID!", 400),
        };
        let token = match ctx.param("token") {
            Some(val) => val,
            None => return Response::error("No delete token!!", 400),
        };
        match kv.get(id).text_with_metadata::<String>().await {
            Ok(val) => {
                if let Some(correct_token) = val.1 {
                    if &correct_token != token {
                        return Response::error("No delete permissions!", 401);
                    }
                } else {
                    return Response::error("No delete token found!", 401);
                }
            }
            Err(e) => {
                console_error!("Failed to get metadata: {e}");
                return Response::error("Failed to get metadata!", 500);
            }
        };
        if let Err(e) = kv.delete(id).await {
            return Response::error(format!("Error deleting paste: {e}"), 500);
        };

        return Response::ok(format!("Deleted paste {id}!"));
    }
    Response::error("Account Misconfigured, no CLOUDPASTE kv found", 500)
}

fn error(err: &str, statuscode: u16) -> Result<Response> {
    let mut context = tera::Context::new();
    context.insert("error", err);
    let mut headers = Headers::new();
    headers.append("Content-Type", "text/html")?;
    if let Ok(resp_html) = tera::Tera::one_off(include_str!("error.html"), &context, true) {
        return Ok(Response::error(resp_html, statuscode)?.with_headers(headers));
    }
    Response::error(err, statuscode)
}
