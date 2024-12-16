use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AppMessage {
    AppError(String),
    DrivesLoaded(Vec<Arc<drives::Device>>),
}
