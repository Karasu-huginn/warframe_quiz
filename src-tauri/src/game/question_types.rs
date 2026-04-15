use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Question {
    pub question_id: u64,
    pub question_type: String,
    pub question_text: String,
    pub clue: Clue,
    pub answers: Vec<Answer>,
    pub time_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Clue {
    Text(String),
    Image(String),
    StatBlock { stats: Vec<(String, String)> },
    TextList(Vec<String>),
    TwoElements { element_a: String, element_b: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct Answer {
    pub index: usize,
    pub text: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnswerResult {
    pub is_correct: bool,
    pub correct_answer_index: usize,
    pub score: u32,
    pub total: u32,
    pub current_streak: u32,
    pub best_streak: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    pub session_id: i64,
    pub score: u32,
    pub total: u32,
    pub current_streak: u32,
    pub best_streak: u32,
}

#[derive(Debug, Clone)]
pub struct StoredQuestion {
    pub question_id: u64,
    pub question_type: String,
    pub correct_answer_index: usize,
}
