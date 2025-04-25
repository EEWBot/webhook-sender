use std::sync::Arc;

pub type Job = crate::request::Request;
pub type JobSender = async_channel::Sender<Job>;
pub type JobReceiver = async_channel::Receiver<Job>;

#[derive(Clone, Debug)]
pub struct Context {
    pub retry_limit: usize,
    pub body: bytes::Bytes,
    pub identity: String,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub context: Arc<Context>,
    pub retry_count: usize,
    pub target: String,
    pub identity: String,
}

impl Request {
    pub fn into_retry(mut self) -> Self {
        self.retry_count += 1;
        self
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        tracing::info!("{} Sent!", self.identity);
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        let count = Arc::strong_count(&self.context);
        match count {
            1000 | 100 | 10 => {
                tracing::info!("{} Last {count: >4}! ", self.context.identity);
            },
            _ => {},
        }
    }
}
