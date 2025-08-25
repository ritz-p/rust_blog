use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]

pub struct CategoryView {
    pub name: String,
    pub slug: String,
}
