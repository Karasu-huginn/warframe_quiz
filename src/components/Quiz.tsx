interface QuizProps {
  timerEnabled: boolean;
  timerSeconds: number;
  onQuit: () => void;
}

export default function Quiz({ timerEnabled: _timerEnabled, timerSeconds: _timerSeconds, onQuit }: QuizProps) {
  return (
    <div className="quiz">
      <p>Quiz — placeholder</p>
      <button className="quit-btn" onClick={onQuit}>Quit</button>
    </div>
  );
}
