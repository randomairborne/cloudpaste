use worker::*;

use crate::{error, NAMESPACE};

pub async fn template(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(kv) = ctx.kv(NAMESPACE) {
        let key = match ctx.param("id") {
            Some(val) => val,
            None => return error("No paste ID!", 400, true),
        };
        let maybe_value = match kv
            .get(key)
            .text_with_metadata::<crate::PasteMetadata>()
            .await
        {
            Ok(val) => val,
            Err(e) => return error(&format!("KV Error: {}", e), 500, true),
        };

        if let (Some(value), Some(meta)) = maybe_value {
            let mut context = tera::Context::new();
            context.insert("id", key);
            context.insert("content", &value);
            context.insert("language", &meta.language);
            if let Ok(page) = tera::Tera::one_off(include_str!("html/paste.html"), &context, true) {
                return Response::from_html(page);
            }

            return error(
                "Templating failed! (this is a bug, github.com/randomairborne/cloudpaste)",
                500,
                true,
            );
        }

        return error("Paste Not Found!", 404, true);
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

pub fn style(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let mut headers = Headers::new();
    headers.append("Content-Type", "text/css")?;
    Ok(Response::ok(include_str!("html/main.css"))?.with_headers(headers))
}

pub fn worker(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let mut headers = Headers::new();
    headers.append("Content-Type", "application/javascript");
    Ok(Response::ok(include_str!("html/worker.js"))?.with_headers(headers))
}
