pub mod product;
pub mod request;
pub mod response;
pub mod scheduler;

pub use scheduler::Scheduler;

pub use scraper::*;
pub use curl::easy::{Easy2, Handler, List};


#[cfg(test)]
mod tests {

    #[test]
    fn simple() {
        assert!(true);
    }
}
