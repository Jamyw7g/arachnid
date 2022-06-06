use curl::easy::{Handler, WriteError};
use scraper::Html;

#[derive(Default)]
pub struct Response {
    url: String,
    headers: Vec<String>,
    buf: Vec<u8>,
}

impl Response {
    pub fn new(url: &str) -> Self {
        let url = url.to_owned();
        Self {
            url,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn get_headers(&self) -> &[String] {
        self.headers.as_ref()
    }

    pub fn get_url(&self) -> &str {
        &self.url
    }

    pub fn set_url(&mut self, url: &str) {
        self.url = url.to_owned();
    }

    pub fn data(&self) -> &[u8] {
        &self.buf
    }

    pub fn html(&self) -> Html {
        let doc = String::from_utf8_lossy(&self.buf);
        Html::parse_document(&doc)
    }
}

impl Handler for Response {
    fn header(&mut self, data: &[u8]) -> bool {
        let line = unsafe { std::str::from_utf8_unchecked(data).trim_end().to_owned() };
        self.headers.push(line);
        true
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
}
