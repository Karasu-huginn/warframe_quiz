pub mod warframe_by_abilities;

use rusqlite::Connection;
use rand::seq::SliceRandom;
use crate::game::question_types::*;

pub fn generate_question(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let types: Vec<fn(&Connection, u64, Option<u32>) -> Result<(Question, StoredQuestion), String>> = vec![
        warframe_by_abilities::generate,
    ];

    let mut indices: Vec<usize> = (0..types.len()).collect();
    indices.shuffle(&mut rand::thread_rng());

    for i in indices {
        match types[i](conn, question_id, time_limit) {
            Ok(result) => return Ok(result),
            Err(_) => continue,
        }
    }
    Err("could not generate any question".to_string())
}

pub fn shuffle_answers(correct: String, wrongs: Vec<String>) -> (Vec<Answer>, usize) {
    let mut all = vec![correct.clone()];
    all.extend(wrongs);
    all.shuffle(&mut rand::thread_rng());
    let correct_index = all.iter().position(|a| *a == correct).unwrap();
    let answers = all.into_iter().enumerate().map(|(i, text)| Answer {
        index: i, text, image: None,
    }).collect();
    (answers, correct_index)
}
