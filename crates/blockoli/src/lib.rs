pub mod blocks;
pub mod embeddings;
pub mod routes;
pub mod vector_store;

use parking_lot::Mutex;
use std::sync::Arc;
use vector_store::vector_store::VectorStore;

pub struct AppState {
    pub vector_store: Arc<Mutex<VectorStore>>,
}
