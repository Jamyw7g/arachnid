pub mod product;
pub mod request;
pub mod response;
pub mod scheduler;

pub use scheduler::Scheduler;

pub use scraper::*;


#[cfg(test)]
mod tests {

    #[test]
    fn simple() {
        assert!(true);
    }
}
