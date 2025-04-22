use std::sync::Arc;

pub type Job = crate::request::Request;
pub type JobSender = async_channel::Sender<Job>;
pub type JobReceiver = async_channel::Receiver<Job>;

#[derive(Clone, Debug)]
pub struct Context {
    pub retry_limit: usize,
    pub body: bytes::Bytes,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub context: Arc<Context>,
    pub retry_count: usize,
    pub target: String,
}

impl Request {
    pub fn into_retry(mut self) -> Self {
        self.retry_count += 1;
        self
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        tracing::info!("OK");
    }
}
