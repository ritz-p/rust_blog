use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]

pub struct TagView {
    pub name: String,
    pub slug: String,
}
