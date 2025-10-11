use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, TransformArgs, TransformContext,
};
use serde_json::json;

const BITPAY_TEST: &str = "https://test.bitpay.com";
const BITPAY_PROD: &str = "https://bitpay.com";
const USE_PROD: bool = false;

thread_local! { static POS_TOKEN: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None); }
pub fn set_pos_token(tok: String) { POS_TOKEN.with(|t| *t.borrow_mut() = Some(tok)); }
fn token() -> String { POS_TOKEN.with(|t| t.borrow().clone()).expect("POS token not set") }
fn base() -> &'static str { if USE_PROD { BITPAY_PROD } else { BITPAY_TEST } }

fn headers() -> Vec<HttpHeader> {
    vec![
        HttpHeader { name: "Content-Type".into(), value: "application/json".into() },
        HttpHeader { name: "x-accept-version".into(), value: "2.0.0".into() },
    ]
}

#[ic_cdk::query]
fn transform(resp: TransformArgs) -> ic_cdk::api::management_canister::http_request::HttpResponse {
    ic_cdk::api::management_canister::http_request::HttpResponse {
        status: resp.response.status, headers: vec![], body: resp.response.body,
    }
}

pub async fn create_invoice(payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let body = json!({
        "token": token(),
        "price": payload["price"],
        "currency": payload["currency"],
        "orderId": payload["orderId"],
        "buyerEmail": payload.get("buyerEmail"),
        "notificationURL": payload["notificationURL"],
        "redirectURL": payload["redirectURL"],
        "itemDesc": payload["itemDesc"],
        "extendedNotifications": true
    }).to_string().into_bytes();

    let arg = CanisterHttpRequestArgument {
        url: format!("{}/invoices", base()),
        method: HttpMethod::POST,
        headers: headers(),
        body: Some(body),
        max_response_bytes: Some(2_000_000),
        transform: Some(TransformContext::from_name("transform", vec![])),
    };
    let (resp,) = http_request(arg, 50_000_000).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let text = String::from_utf8(resp.body)?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    Ok(v["data"].clone())
}

pub async fn get_invoice(id: &str) -> anyhow::Result<serde_json::Value> {
    let url = format!("{}/invoices/{}?token={}", base(), id, urlencoding::encode(&token()));
    let arg = CanisterHttpRequestArgument {
        url, method: HttpMethod::GET, headers: headers(), body: None,
        max_response_bytes: Some(1_000_000),
        transform: Some(TransformContext::from_name("transform", vec![])),
    };
    let (resp,) = http_request(arg, 30_000_000).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let text = String::from_utf8(resp.body)?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    Ok(v["data"].clone())
}