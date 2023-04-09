use serde::Deserialize;

/// Meta Section in Tito API Response
#[derive(Debug, Deserialize)]
pub struct Meta {
    pub current_page: u32,
    pub next_page: Option<u32>,
    pub prev_page: Option<u32>,
    pub total_pages: u32,
    pub total_count: u32,
    pub per_page: u32,
    pub overall_total: u32,
}
