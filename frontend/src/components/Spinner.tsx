export default function Spinner({ className = "h-5 w-5" }: { className?: string }) {
  return (
    <div
      className={`block h-5 w-5 animate-spin rounded-full border-2 border-slate-300 border-t-indigo-600 ${className}`}
      role="status"
      aria-label="Loading"
    />
  );
}