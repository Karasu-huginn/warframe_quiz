pub mod wiki_client;
pub mod lua_parser;

#[derive(Debug, Default)]
pub struct CategoryReport {
    pub category: String,
    pub inserted: usize,
    pub failed: usize,
}

#[derive(Debug)]
pub struct ImageTask {
    pub wiki_filename: String,
    pub local_subdir: String,
}

pub struct CategoryResult {
    pub report: CategoryReport,
    pub images: Vec<ImageTask>,
}
