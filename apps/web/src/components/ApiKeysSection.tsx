import type { ApiKey, Project } from "./types";

type ApiKeysSectionProps = {
  keyName: string;
  keyPermission: string;
  keyOnce: string | null;
  selectedKeyProject: string;
  projects: Project[];
  keys: ApiKey[];
  hasProjects: boolean;
  onKeyNameChange: (value: string) => void;
  onKeyPermissionChange: (value: string) => void;
  onSelectedProjectChange: (value: string) => void;
  onCreateKey: () => void | Promise<void>;
  onRevokeKey: (id: string) => void | Promise<void>;
};

export function ApiKeysSection({
  keyName,
  keyPermission,
  keyOnce,
  selectedKeyProject,
  projects,
  keys,
  hasProjects,
  onKeyNameChange,
  onKeyPermissionChange,
  onSelectedProjectChange,
  onCreateKey,
  onRevokeKey,
}: ApiKeysSectionProps) {
  return (
    <>
      <h2>API keys</h2>
      <div className="card">
        <label>
          Name
          <input value={keyName} onChange={event => onKeyNameChange(event.target.value)} placeholder="CI deploy" />
        </label>
        <label>
          Project
          <select value={selectedKeyProject} onChange={event => onSelectedProjectChange(event.target.value)} disabled={!hasProjects}>
            {projects.map(project => (
              <option key={project.id} value={project.slug}>
                {project.slug}
              </option>
            ))}
          </select>
        </label>
        <label>
          Permission
          <select value={keyPermission} onChange={event => onKeyPermissionChange(event.target.value)}>
            <option value="read">read</option>
            <option value="write">write</option>
          </select>
        </label>
        <button type="button" onClick={() => void onCreateKey()} disabled={!hasProjects}>
          Create API key
        </button>
        {keyOnce && (
          <div>
            <p className="muted">Copy this key now. It won't be shown again.</p>
            <div id="key-once">{keyOnce}</div>
          </div>
        )}
        <ul>
          {keys.map(key => (
            <li key={key.id}>
              <span>
                <code>{key.key_prefix}…</code> {key.name}
              </span>
              <button className="danger" type="button" onClick={() => void onRevokeKey(key.id)}>
                Revoke
              </button>
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}
