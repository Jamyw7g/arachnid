use std::sync::{mpsc::SyncSender, Arc};

use crate::{product::Product, response::Response};

pub type CallBack = Box<dyn FnOnce(Response, Arc<SyncSender<Product>>) + Send + 'static>;

pub struct Request {
    pub(crate) url: String,
    pub(crate) callback: CallBack,
}

impl Request {
    pub fn new<F>(url: &str, callback: F) -> Self
    where
        F: FnOnce(Response, Arc<SyncSender<Product>>) + Send + 'static,
    {
        let url = url.to_owned();
        let callback = Box::new(callback);
        Self { url, callback }
    }
}
