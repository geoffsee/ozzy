import type { Me } from "./types";

type AccountCardProps = {
  me: Me;
  onLogout: () => void | Promise<void>;
};

export function AccountCard({ me, onLogout }: AccountCardProps) {
  return (
    <div className="card">
      <div>
        <strong>{me.login}</strong>
        <div className="muted">{me.name || ""}</div>
      </div>
      <button type="button" className="secondary" onClick={() => void onLogout()}>
        Sign out
      </button>
    </div>
  );
}
