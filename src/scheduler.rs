use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};
use std::time::Duration;

use curl::easy::Easy2;
use curl::multi::{Easy2Handle, Multi};
use threadpool::ThreadPool;

use crate::product::{MidProduct, Product, Value};
use crate::request::Request;
use crate::response::Response;

pub type Pipeline =
    Box<dyn Fn(HashMap<String, Value>) -> Option<HashMap<String, Value>> + Send + Sync + 'static>;
pub type Middleware = Box<dyn Fn(Request) -> MidProduct + Send + Sync + 'static>;

pub static TOKEN: AtomicUsize = AtomicUsize::new(0);

pub fn next_token() -> usize {
    TOKEN.fetch_add(1, Ordering::SeqCst)
}

pub struct Scheduler {
    multi: Multi,
    pool: ThreadPool,
    handles: HashMap<usize, (Easy2Handle<Response>, Request)>,
    request_queue: Arc<Mutex<Vec<MidProduct>>>,
    product_tx: Arc<SyncSender<Product>>,
    finished: Arc<AtomicBool>,
    timeout: Option<Duration>,
}

impl Scheduler {
    pub fn new(
        pipelines: Vec<Pipeline>,
        middlewares: Vec<Middleware>,
        timeout: Option<Duration>,
    ) -> anyhow::Result<Self> {
        let request_queue = Arc::new(Mutex::new(Vec::new()));
        let (product_tx, product_rx) = sync_channel(64);
        let product_tx = Arc::new(product_tx);
        let pool = threadpool::Builder::new()
            .thread_name("Pool".into())
            .build();
        let multi = Multi::new();
        let handles = HashMap::new();
        let finished = Arc::new(AtomicBool::new(false));

        std::thread::Builder::new().name("Transfer".into()).spawn({
            let request_queue = Arc::clone(&request_queue);
            let finished = Arc::clone(&finished);
            let pool = threadpool::Builder::new()
                .thread_name("Middlewares".into())
                .build();
            let pipelines = Arc::new(pipelines);
            let middlewares = Arc::new(middlewares);
            move || loop {
                if let Ok(val) = product_rx.recv_timeout(timeout.unwrap_or(Duration::from_secs(30)))
                {
                    match val {
                        Product::Request(req) => {
                            let request_queue = Arc::clone(&request_queue);
                            let middlewares = Arc::clone(&middlewares);
                            pool.execute(move || {
                                let mut mid_product = MidProduct::Request(req);
                                for mid in middlewares.iter() {
                                    match mid_product {
                                        MidProduct::Request(req) => mid_product = mid(req),
                                        _ => break,
                                    }
                                }
                                match mid_product {
                                    MidProduct::Ignore => (),
                                    _ => {
                                        let mut queue = request_queue.lock().unwrap();
                                        queue.push(mid_product);
                                    }
                                }
                            });
                        }
                        Product::Item(item) => {
                            let pipelines = Arc::clone(&pipelines);
                            pool.execute(move || {
                                let mut item = Some(item);
                                for pipe in pipelines.iter() {
                                    if let Some(val) = item.take() {
                                        item = pipe(val);
                                    } else {
                                        break;
                                    }
                                }
                                if let Some(val) = item {
                                    log::info!("Item: {:?}", val);
                                }
                            });
                        }
                        Product::Finished => finished.store(true, Ordering::Relaxed),
                    }
                } else {
                    log::info!("No product.");
                    if pool.active_count() == 0 && pool.queued_count() == 0 {
                        log::info!("Scheduler prepare to stop.");
                        finished.store(true, Ordering::Relaxed);
                    }
                }
            }
        })?;

        log::info!("Init scheduler");
        Ok(Self {
            multi,
            pool,
            handles,
            request_queue,
            product_tx,
            finished,
            timeout,
        })
    }

    pub fn start_request(&self, request: Request) {
        self.product_tx
            .send(Product::Request(request))
            .expect("Fail to start request");
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut alive = true;
        while alive {
            let mut queue = self.request_queue.lock().unwrap();
            let size = self.pool.max_count() * 2;
            let len = if self.handles.len() >= size {
                0
            } else {
                (size - self.handles.len()).min(queue.len())
            };
            let que_res = Vec::from_iter(queue.drain(..len));
            drop(queue);

            if !que_res.is_empty() {
                for req in que_res.into_iter() {
                    match req {
                        MidProduct::Request(req) => {
                            let mut easy = Easy2::new(Response::default());
                            easy.url(&req.url)?;
                            easy.get(true)?;
                            log::info!("New request: {}", &req.url);
                            let mut handler = self.multi.add2(easy)?;
                            let token = next_token();
                            handler.set_token(token)?;
                            self.handles.insert(token, (handler, req));
                        }
                        MidProduct::Easy((easy, req)) => {
                            log::info!("New easy2: {}", &req.url);
                            let mut handler = self.multi.add2(easy)?;
                            let token = next_token();
                            handler.set_token(token)?;
                            self.handles.insert(token, (handler, req));
                        }
                        MidProduct::Response((resp, cb)) => {
                            let tx = Arc::clone(&self.product_tx);
                            self.pool.execute(move || {
                                cb(resp, tx);
                            });
                        }
                        _ => (),
                    }
                }
            }
            if self.multi.perform()? == 0 {
                if self.finished.load(Ordering::Acquire) {
                    log::info!("Shutdowning spider.");
                    alive = false;
                } else {
                    std::thread::sleep(Duration::from_millis(500));
                }
            }

            self.multi.messages(|msg| {
                let token = msg.token().expect("Fail to get token");
                let (mut handler, req) = self.handles.remove(&token).expect("Unexpect token.");
                let Request { url, callback } = req;
                match msg.result_for2(&handler).unwrap() {
                    Ok(_) => {
                        let mut response =
                            std::mem::replace(handler.get_mut(), Response::default());
                        if let Some(url) = handler.redirect_url().unwrap() {
                            response.set_url(url);
                        } else {
                            response.set_url(&url);
                        }
                        let tx = Arc::clone(&self.product_tx);
                        self.pool.execute(move || {
                            callback(response, tx);
                        });
                    }
                    Err(e) => {
                        log::error!("Error: {} - {}", e, url);
                    }
                }
            });

            if alive {
                self.multi
                    .wait(&mut [], self.timeout.unwrap_or(Duration::from_secs(15)))?;
            }
        }
        Ok(())
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.pool.join();
    }
}
