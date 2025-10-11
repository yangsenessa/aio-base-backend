use crate::types::{Order, OrderStatus};
use candid::CandidType;
use ic_stable_structures::memory_manager::{MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MM: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static ORDERS: RefCell<StableBTreeMap<String, Order, Memory>> = RefCell::new(
        StableBTreeMap::init(MM.with(|m| m.borrow().get(0)))
    );
}

impl Storable for Order {
    fn to_bytes(&self) -> Cow<[u8]> { Cow::Owned(candid::encode_one(self).unwrap()) }
    fn from_bytes(bytes: Cow<[u8]>) -> Self { candid::decode_one(&bytes).unwrap() }
}

pub fn now_ns() -> u64 { ic_cdk::api::time() }

pub fn get(order_id: &str) -> Option<Order> { ORDERS.with(|m| m.borrow().get(order_id)) }

pub fn put(o: Order) { ORDERS.with(|m| { m.borrow_mut().insert(o.order_id.clone(), o); }); }

pub fn upsert_patch(order_id: &str, f: impl FnOnce(&mut Order)) -> Order {
    ORDERS.with(|m| {
        let mut map = m.borrow_mut();
        let mut o = map.get(order_id).unwrap_or_else(|| Order{
            order_id: order_id.to_string(),
            amount: 0.0,
            currency: "USD".into(),
            buyer_email: None,
            shipping_address: "".into(),
            sku: "".into(),
            bitpay_invoice_id: None,
            bitpay_invoice_url: None,
            status: OrderStatus::Created,
            shipment_no: None,
            created_at_ns: now_ns(),
            updated_at_ns: now_ns(),
        });
        f(&mut o);
        o.updated_at_ns = now_ns();
        map.insert(order_id.to_string(), o.clone());
        o
    })
}