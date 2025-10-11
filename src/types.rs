use candid::{CandidType, Deserialize};

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum OrderStatus { Created, New, Paid, Confirmed, Complete, Expired, Invalid, Delivered }

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct CreateOrderArgs {
    pub order_id: String,
    pub amount: f64,
    pub currency: String,
    pub buyer_email: Option<String>,
    pub shipping_address: String,
    pub sku: String,
    pub redirect_base: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Order {
    pub order_id: String,
    pub amount: f64,
    pub currency: String,
    pub buyer_email: Option<String>,
    pub shipping_address: String,
    pub sku: String,
    pub bitpay_invoice_id: Option<String>,
    pub bitpay_invoice_url: Option<String>,
    pub status: OrderStatus,
    pub shipment_no: Option<String>,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct InvoiceResp { pub invoice_id: String, pub invoice_url: String }
