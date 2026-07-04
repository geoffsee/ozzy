import type { Me } from "./types";

type AuthPromptCardProps = {
  authChecked: boolean;
  me: Me | null;
};

export function AuthPromptCard({ authChecked, me }: AuthPromptCardProps) {
  if (!authChecked || me) {
    return null;
  }

  return (
    <div className="card">
      <p className="muted">Sign in with GitHub to manage projects, API keys, and secrets.</p>
      <a href="/auth/github">
        <button type="button">Sign in with GitHub</button>
      </a>
    </div>
  );
}
