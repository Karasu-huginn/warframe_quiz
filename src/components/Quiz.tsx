import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Question, AnswerResult, Clue } from "../types";

interface QuizProps {
  timerEnabled: boolean;
  timerSeconds: number;
  onQuit: () => void;
}

export default function Quiz({ timerEnabled, timerSeconds, onQuit }: QuizProps) {
  const [question, setQuestion] = useState<Question | null>(null);
  const [result, setResult] = useState<AnswerResult | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [score, setScore] = useState(0);
  const [total, setTotal] = useState(0);
  const [streak, setStreak] = useState(0);
  const [_bestStreak, setBestStreak] = useState(0);
  const [loading, setLoading] = useState(true);
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const startTime = useRef<number>(Date.now());
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const resultRef = useRef<AnswerResult | null>(null);

  const fetchQuestion = async () => {
    setResult(null);
    resultRef.current = null;
    setSelectedIndex(null);
    setLoading(true);
    try {
      const q = await invoke<Question>("next_question");
      setQuestion(q);
      setImageSrc(null);
      // Load image if clue is Image type
      if (q.clue.type === "Image") {
        const relativePath = q.clue.data.replace(/^assets\//, "");
        invoke<string>("get_asset_base64", { relativePath })
          .then(setImageSrc)
          .catch(() => setImageSrc(null));
      }
      startTime.current = Date.now();
      if (timerEnabled) {
        if (timerRef.current) clearTimeout(timerRef.current);
        timerRef.current = setTimeout(() => {
          handleAnswer(-1);
        }, timerSeconds * 1000);
      }
    } catch (e) {
      console.error("Failed to get question:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchQuestion();
    return () => { if (timerRef.current) clearTimeout(timerRef.current); };
  }, []);

  const handleAnswer = async (answerIndex: number) => {
    if (resultRef.current) return; // Use ref to avoid stale closure
    if (timerRef.current) clearTimeout(timerRef.current);
    const elapsed = (Date.now() - startTime.current) / 1000;
    const idx = answerIndex < 0 ? 0 : answerIndex;
    setSelectedIndex(idx);
    try {
      const res = await invoke<AnswerResult>("submit_answer", {
        answerIndex: idx,
        elapsedSeconds: elapsed,
      });
      resultRef.current = res;
      setResult(res);
      setScore(res.score);
      setTotal(res.total);
      setStreak(res.current_streak);
      setBestStreak(res.best_streak);
    } catch (e) {
      console.error("Failed to submit:", e);
    }
  };

  const handleNext = () => { fetchQuestion(); };

  const getButtonClass = (idx: number): string => {
    if (!result) return "answer-btn";
    if (idx === result.correct_answer_index) return "answer-btn correct";
    if (idx === selectedIndex && !result.is_correct) return "answer-btn wrong";
    return "answer-btn dimmed";
  };

  if (loading && !question) return <div className="quiz"><p className="loading">Chargement...</p></div>;
  if (!question) return null;

  // Strip wiki markup tags like <DT_PUNCTURE_COLOR>...</DT_PUNCTURE_COLOR>
  const stripTags = (text: string) => text.replace(/<[^>]+>/g, "");

  return (
    <div className="quiz">
      <div className="quiz-top">
        <span className="quiz-score">{score}/{total}</span>
        <span className="quiz-streak">{streak > 0 ? `${streak} 🔥` : "0"}</span>
        <button className="quit-btn" onClick={onQuit}>Quitter</button>
      </div>

      {timerEnabled && !result && (
        <div className="timer-bar-container">
          <div className="timer-bar" style={{ animationDuration: `${timerSeconds}s` }} key={question.question_id} />
        </div>
      )}

      <div className="quiz-question">{question.question_text}</div>
      <div className="clue-box">{renderClue(question.clue, imageSrc)}</div>

      <div className="answers">
        {question.answers.map((a) => (
          <button key={a.index} className={getButtonClass(a.index)} onClick={() => handleAnswer(a.index)} disabled={!!result}>
            {stripTags(a.text)}
          </button>
        ))}
      </div>

      {result && <button className="next-btn" onClick={handleNext}>Suivant</button>}
    </div>
  );
}

function renderClue(clue: Clue, imageSrc: string | null): JSX.Element {
  const strip = (text: string) => text.replace(/<[^>]+>/g, "");
  switch (clue.type) {
    case "Text":
      return <p className="clue-text">{strip(clue.data)}</p>;
    case "TextList":
      return <ul className="clue-list">{clue.data.map((item, i) => <li key={i}>{strip(item)}</li>)}</ul>;
    case "Image":
      return imageSrc
        ? <img className="clue-image" src={imageSrc} alt="clue" />
        : <p className="clue-text">Image non disponible</p>;
    case "StatBlock":
      return (
        <table className="clue-stats"><tbody>
          {clue.data.stats.map(([label, value], i) => <tr key={i}><td>{label}</td><td>{value}</td></tr>)}
        </tbody></table>
      );
    case "TwoElements":
      return (
        <div className="clue-elements">
          <span className="element-pill">{clue.data.element_a}</span>
          <span className="element-plus">+</span>
          <span className="element-pill">{clue.data.element_b}</span>
          <span className="element-equals">= ?</span>
        </div>
      );
  }
}
