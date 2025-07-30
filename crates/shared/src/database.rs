use std::sync::Arc;

use dashmap::DashMap;

use crate::{domain::User, service::UserServiceImpl};

pub type Database = Arc<DashMap<String, User>>;
pub type SharedState = Arc<UserServiceImpl>;
