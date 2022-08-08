use worker::*;

use crate::{error, MAX_UPLOAD_BYTES, NAMESPACE, SECONDS_IN_A_WEEK};

pub async fn new(mut req: Request, ctx: RouteContext<()>, is_raw_form: bool) -> Result<Response> {
    if let Ok(kv) = ctx.kv(NAMESPACE) {
        let id = crate::randstr(10);
        let revoke = crate::randstr(64);
        let form = match req.form_data().await {
            Ok(val) => val,
            Err(e) => {
                return error(
                    &format!("Failed to get data from POST request: {e}"),
                    400,
                    is_raw_form,
                )
            }
        };
        let data = if let Some(data) = form.get("contents") {
            match data {
                FormEntry::Field(f) => {
                    if f.is_empty() {
                        return error("Pastes must have at least one character!", 400, is_raw_form);
                    } else if f.len() > MAX_UPLOAD_BYTES {
                        return error(
                            &format!("Pastes must be less then {MAX_UPLOAD_BYTES} bytes!"),
                            400,
                            is_raw_form,
                        );
                    }
                    f
                }
                FormEntry::File(_) => {
                    return error("File uploads are not supported yet.", 501, is_raw_form)
                }
            }
        } else {
            return error("Failed to get content from form-data", 400, is_raw_form);
        };
        let put = match kv.put(&id, data) {
            Ok(val) => val,
            Err(e) => return error(&format!("KV error: {e}"), 500, is_raw_form),
        };
        let meta = match put.metadata(&revoke) {
            Ok(val) => val,
            Err(e) => {
                return error(
                    &format!("Failed to serialize metadata: {e}"),
                    500,
                    is_raw_form,
                )
            }
        };
        if let Err(e) = meta.expiration_ttl(SECONDS_IN_A_WEEK).execute().await {
            return error(&format!("Failed to insert into KV: {e}"), 500, is_raw_form);
        }
        if is_raw_form {
            let mut headers = Headers::new();
            headers.append("Location", &format!("/{id}")).ok();
            if let Ok(resp) = Response::ok("") {
                return Ok(resp.with_headers(headers).with_status(302));
            }
        } else {
            return Response::from_json(&serde_json::json!({"id": id, "revoke": revoke}));
        }
    };
    error(
        "Account Misconfigured, no CLOUDPASTE kv found",
        500,
        is_raw_form,
    )
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
