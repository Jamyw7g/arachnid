use std::{
    collections::HashMap,
    sync::{mpsc::SyncSender, Arc},
};

use arachnid::{
    product::{Item, Product, Value, MidProduct},
    request::Request,
    response::Response,
    scheduler::{Scheduler, Pipeline, Middleware},
    Selector, Easy2, List
};

fn main() -> anyhow::Result<()> {
    let pipe: Vec<Pipeline> = vec![Box::new(pipe_show)];
    let mids: Vec<Middleware> = vec![Box::new(header_mid)];
    let mut scheduler = Scheduler::new(pipe, mids, None)?;
    for idx in 1..3 {
        let url = format!(
            "https://www.zcool.com.cn/discover?cate=33&subCate=34&page={}",
            idx
        );
        let req = Request::new(&url, page_parse);
        scheduler.start_request(req);
    }

    scheduler.run()
}

fn header_mid(req: Request) -> MidProduct {
    let mut easy = Easy2::new(Response::default());
    easy.url(&req.url).unwrap();
    let mut headers = List::new();
    headers.append("User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.5 Safari/605.1.15").unwrap();
    easy.http_headers(headers).unwrap();

    MidProduct::Easy((easy, req))
}

fn pipe_show(item: Item) -> Option<Item> {
    println!("{:?}", item);
    None
}

fn page_parse(resp: Response, tx: Arc<SyncSender<Product>>) {
    let css = Selector::parse("div.contentCardBox a.cardImgHover").unwrap();
    for ele in resp.html().select(&css) {
        if let Some(href) = ele.value().attr("href") {
            let req = Request::new(href, item_parse);
            tx.send(Product::Request(req)).unwrap();
        }
    }
}

fn item_parse(resp: Response, tx: Arc<SyncSender<Product>>) {
    let css = Selector::parse("div.photoInformationContent img").unwrap();
    for ele in resp.html().select(&css) {
        if let Some(src) = ele.value().attr("src") {
            let req = Request::new(src, save_img);
            tx.send(Product::Request(req)).unwrap();
        }
    }
}

fn save_img(resp: Response, tx: Arc<SyncSender<Product>>) {
    let url = resp.get_url().to_owned();
    let map = HashMap::from([("img".to_owned(), Value::Str(url))]);
    tx.send(Product::Item(map)).unwrap();
    /*
    let filename = resp
        .get_url()
        .split_once('?')
        .unwrap_or(("", ""))
        .0
        .split('/')
        .last()
        .unwrap_or("");
    if filename.is_empty() {
        return;
    }

    let filename = format!("./imgs/{}", filename);
    let mut fp = File::create(&filename).unwrap();
    fp.write(resp.data()).unwrap();
    println!("{filename}");
     */
}

