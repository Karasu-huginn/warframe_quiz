export interface Question {
  question_id: number;
  question_type: string;
  question_text: string;
  clue: Clue;
  answers: Answer[];
  time_limit: number | null;
}

export type Clue =
  | { type: "Text"; data: string }
  | { type: "Image"; data: string }
  | { type: "StatBlock"; data: { stats: [string, string][] } }
  | { type: "TextList"; data: string[] }
  | { type: "TwoElements"; data: { element_a: string; element_b: string } };

export interface Answer {
  index: number;
  text: string;
  image: string | null;
}

export interface AnswerResult {
  is_correct: boolean;
  correct_answer_index: number;
  score: number;
  total: number;
  current_streak: number;
  best_streak: number;
}

export interface SessionStats {
  session_id: number;
  score: number;
  total: number;
  current_streak: number;
  best_streak: number;
}

export interface OverallStats {
  total_games: number;
  best_streak: number;
  total_correct: number;
  total_answered: number;
}

export interface RecentSession {
  id: number;
  started_at: string;
  score: number;
  total_questions: number;
  best_streak: number;
}

export interface FetchProgress {
  category: string;
  status: string;
  current: number;
  total: number;
  message: string;
}

export type Screen = "home" | "quiz" | "settings" | "stats";
export type Theme = "duviri" | "grineer" | "lotus";
