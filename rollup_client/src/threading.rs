use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct SharedState {
    pub counter: i32,
    // Add other fields you need to share between threads, such as:
    // pub config: HashMap<String, String>,
    // pub transaction_queue: VecDeque<Transaction>,
    // etc.
}

pub type SharedStateHandle = Arc<RwLock<SharedState>>;

impl SharedState {
    // Add methods for common operations on shared state
    pub fn increment_counter(&mut self) {
        self.counter += 1;
    }
}

pub fn create_shared_state() -> SharedStateHandle {
    Arc::new(RwLock::new(SharedState {
        counter: 0,
    }))
}