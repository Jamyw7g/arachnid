use std::{
    collections::HashMap,
    sync::{mpsc::SyncSender, Arc},
};

use arachnid::{
    product::{Product, Value},
    request::Request,
    response::Response,
    scheduler::{Scheduler, PipeLine},
    Selector
};

fn main() -> anyhow::Result<()> {
    let pipe: Vec<PipeLine> = vec![];
    let mut scheduler = Scheduler::new(pipe, None)?;
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

