use std::{sync::mpsc::SyncSender, collections::HashMap};
use std::sync::Arc;

use arachnid::Scheduler;
use arachnid::product::Value;
use arachnid::scheduler::{Pipeline, Middleware};
use arachnid::{product::{Item, MidProduct, Product}, request::Request, Easy2, response::Response, Selector};


fn main() {
    let pipes: Vec<Pipeline> = vec![Box::new(pipe_show)];
    let mids: Vec<Middleware> = vec![Box::new(proxy_mid)];

    let mut scheduler = Scheduler::new(pipes, mids, None).unwrap();
    let req = Request::new("http://tape6m4x7swc7lwx2n2wtyccu4lt2qyahgwinx563gqfzeedn5nb4gid.onion/", page_parse);
    scheduler.start_request(req);

    scheduler.run().unwrap();
}

fn page_parse(resp: Response, tx: Arc<SyncSender<Product>>) {
    let title_css = Selector::parse("title").unwrap();
    let mut item = HashMap::new();
    for ele in resp.html().select(&title_css) {
        let title = ele.inner_html();
        let val = if title.is_empty() { Value::Nil } else { Value::Str(title) };
        item.insert("title".into(), val);
    }
    tx.send(Product::Item(item)).unwrap();

    let a_css = Selector::parse("a").unwrap();
    for ele in resp.html().select(&a_css) {
        if let Some(val) = ele.value().attr("href") {
            let req = Request::new(val, page_parse);
            tx.send(Product::Request(req)).unwrap();
        }
    }
}


fn pipe_show(item: Item) -> Option<Item> {
    println!("{:?}", item);
    None
}

fn proxy_mid(req: Request) -> MidProduct {
    let mut easy = Easy2::new(Response::default());
    easy.url(&req.url).unwrap();
    easy.proxy("http://127.0.0.1:8118").unwrap();
    easy.get(true).unwrap();

    MidProduct::Easy((easy, req))
}