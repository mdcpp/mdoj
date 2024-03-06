use anyhow::{anyhow, Result};
use cfg_if::cfg_if;
use cookie::{Cookie, CookieJar};
use leptos::*;

pub const TOKEN_COOKIE: &str = "token";

pub fn cookie() -> Result<CookieJar> {
    cfg_if! {if #[cfg(feature = "ssr")] {
        use actix_web::http::header::COOKIE;
        use leptos_actix::ResponseOptions;
        let resp: ResponseOptions = expect_context();
        let guard = resp.0.read();
        let value = guard
            .headers
            .get(COOKIE)
            .map(|v| v.to_str().map(|v| v.to_owned()))
            .unwrap_or(Ok("".to_owned()))?;
    } else {
        use wasm_bindgen::JsCast;
        // `Document` can cast to `HtmlDocument`
        let doc = gloo::utils::document()
            .dyn_into::<web_sys::HtmlDocument>()
            .unwrap();
        let value = doc.cookie().map_err(|_| anyhow!("cannot find cookie in js api"))?;
    }}

    let mut jar = CookieJar::new();
    for result in Cookie::split_parse(value) {
        let cookie = result?;
        jar.add_original(cookie);
    }
    Ok(jar)
}

pub fn set_cookie(jar: &CookieJar) -> Result<()> {
    cfg_if! {if #[cfg(feature="ssr")] {
        use actix_web::http::header::SET_COOKIE;
        use leptos_actix::ResponseOptions;
        let resp: ResponseOptions = expect_context();
        for c in jar.delta() {
            resp.append_header(SET_COOKIE, c.to_string().try_into()?);
        }
    } else {
        use wasm_bindgen::JsCast;
        let value = jar
            .iter()
            .map(|c| c.to_string())
            .reduce(|a, b| format!("{a};{b}"))
            .unwrap_or_default();
        // `Document` can cast to `HtmlDocument`
        let doc = gloo::utils::document()
            .dyn_into::<web_sys::HtmlDocument>()
            .unwrap();
        doc.set_cookie(&value)
            .map_err(|_| anyhow!("cannot find cookie in js api"))?;
    }}

    Ok(())
}
