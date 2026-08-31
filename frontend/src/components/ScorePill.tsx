import { Badge, type Tone } from "./Badge";

function scoreTone(score: number): Tone {
  if (score >= 80) return "emerald";
  if (score >= 60) return "amber";
  if (score >= 40) return "sky";
  return "slate";
}

export default function ScorePill({ score }: { score: number }) {
  return <Badge tone={scoreTone(score)}>{score}</Badge>;
}