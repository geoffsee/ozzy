import type { Project, SecretListItem } from "./types";

type SecretsSectionProps = {
  selectedSecretProject: string;
  secretKey: string;
  secretValue: string;
  projects: Project[];
  secrets: SecretListItem[];
  hasProjects: boolean;
  onSelectedProjectChange: (value: string) => void;
  onSecretKeyChange: (value: string) => void;
  onSecretValueChange: (value: string) => void;
  onRevealSecret: (slug: string, key: string) => void | Promise<void>;
  onDeleteSecret: (slug: string, key: string) => void | Promise<void>;
  onSaveSecret: () => void | Promise<void>;
};

export function SecretsSection({
  selectedSecretProject,
  secretKey,
  secretValue,
  projects,
  secrets,
  hasProjects,
  onSelectedProjectChange,
  onSecretKeyChange,
  onSecretValueChange,
  onRevealSecret,
  onDeleteSecret,
  onSaveSecret,
}: SecretsSectionProps) {
  return (
    <>
      <h2>Secrets</h2>
      <div className="card">
        <label>
          Project
          <select value={selectedSecretProject} onChange={event => onSelectedProjectChange(event.target.value)} disabled={!hasProjects}>
            {projects.map(project => (
              <option key={project.id} value={project.slug}>
                {project.slug}
              </option>
            ))}
          </select>
        </label>
        <ul>
          {secrets.map(secret => (
            <li key={secret.key_name}>
              <span>
                <code>{secret.key_name}</code> v{secret.version}
              </span>
              <span>
                <button className="secondary" type="button" onClick={() => void onRevealSecret(selectedSecretProject, secret.key_name)}>
                  Reveal
                </button>{" "}
                <button className="danger" type="button" onClick={() => void onDeleteSecret(selectedSecretProject, secret.key_name)}>
                  Delete
                </button>
              </span>
            </li>
          ))}
        </ul>
        <div className="row">
          <div>
            <label>
              Key
              <input value={secretKey} onChange={event => onSecretKeyChange(event.target.value)} placeholder="DATABASE_URL" />
            </label>
          </div>
          <div>
            <label>
              Value
              <input value={secretValue} onChange={event => onSecretValueChange(event.target.value)} type="password" placeholder="value" />
            </label>
          </div>
        </div>
        <button type="button" onClick={() => void onSaveSecret()} disabled={!hasProjects}>
          Save secret
        </button>
      </div>
    </>
  );
}
