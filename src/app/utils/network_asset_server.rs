use std::collections::HashMap;

pub struct NetworkAssetServer {
    pub max_concurrent: usize,
    store: HashMap<String, Vec<u8>>,
}