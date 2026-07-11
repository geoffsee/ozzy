import "./index.css";
import { useEffect, useState } from "react";
import { AccountCard } from "./components/AccountCard";
import { ApiKeysSection } from "./components/ApiKeysSection";
import { AuthPromptCard } from "./components/AuthPromptCard";
import { ProjectsSection } from "./components/ProjectsSection";
import { SecretsSection } from "./components/SecretsSection";
import { StatusMessage } from "./components/StatusMessage";
import type { ApiKey, Me, MessageState, Project, SecretListItem, SecretValue } from "./components/types";
import { track } from "./telemetry";

async function api<T>(path: string, opts: RequestInit = {}): Promise<T> {
  const headers = new Headers(opts.headers);
  if (opts.body) {
    headers.set("Content-Type", "application/json");
  }

  const res = await fetch(path, {
    credentials: "include",
    ...opts,
    headers,
  });

  if (!res.ok) {
    const errorBody = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(errorBody.error || res.statusText);
  }

  if (res.status === 204) {
    return null as T;
  }

  return res.json() as Promise<T>;
}

const mutatingMethods = new Set(["POST", "PUT", "PATCH", "DELETE"]);

export function App() {
  const [csrfToken, setCsrfToken] = useState<string | null>(null);
  const [authChecked, setAuthChecked] = useState(false);
  const [me, setMe] = useState<Me | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [secrets, setSecrets] = useState<SecretListItem[]>([]);
  const [selectedKeyProject, setSelectedKeyProject] = useState("");
  const [selectedSecretProject, setSelectedSecretProject] = useState("");
  const [projectSlug, setProjectSlug] = useState("");
  const [projectName, setProjectName] = useState("");
  const [keyName, setKeyName] = useState("");
  const [keyPermission, setKeyPermission] = useState("read");
  const [keyOnce, setKeyOnce] = useState<string | null>(null);
  const [secretKey, setSecretKey] = useState("");
  const [secretValue, setSecretValue] = useState("");
  const [message, setMessage] = useState<MessageState>({ text: "", isError: false });

  const hasProjects = projects.length > 0;

  const apiRequest = async <T,>(path: string, opts: RequestInit = {}) => {
    const method = (opts.method ?? "GET").toUpperCase();
    const headers = new Headers(opts.headers);

    if (csrfToken && mutatingMethods.has(method)) {
      headers.set("X-CSRF-Token", csrfToken);
    }

    return api<T>(path, {
      ...opts,
      headers,
    });
  };

  const setSuccessMessage = (text: string) => setMessage({ text, isError: false });
  const setErrorMessage = (error: unknown) => setMessage({ text: error instanceof Error ? error.message : String(error), isError: true });

  const refreshProjects = async () => {
    const loadedProjects = await apiRequest<Project[]>("/api/projects");
    setProjects(loadedProjects);

    if (!loadedProjects.length) {
      setSelectedKeyProject("");
      setSelectedSecretProject("");
      setSecrets([]);
      return;
    }

    setSelectedKeyProject(prev => {
      if (loadedProjects.some(project => project.slug === prev)) {
        return prev;
      }
      return loadedProjects[0]!.slug;
    });

    setSelectedSecretProject(prev => {
      if (loadedProjects.some(project => project.slug === prev)) {
        return prev;
      }
      return loadedProjects[0]!.slug;
    });
  };

  const refreshKeys = async () => {
    const loadedKeys = await apiRequest<ApiKey[]>("/api/keys");
    setKeys(loadedKeys);
  };

  const refreshSecrets = async (slug: string) => {
    if (!slug) {
      setSecrets([]);
      return;
    }
    const loadedSecrets = await apiRequest<SecretListItem[]>("/api/secrets/list", {
      method: "POST",
      body: JSON.stringify({ project: slug }),
    });
    setSecrets(loadedSecrets);
  };

  useEffect(() => {
    void (async () => {
      try {
        const loadedMe = await api<Me>("/api/me");
        setMe(loadedMe);
        const csrf = await api<{ token: string }>("/api/csrf");
        setCsrfToken(csrf.token);
        track("session_loaded");
        await refreshProjects();
        await refreshKeys();
      } catch {
        setMe(null);
        setCsrfToken(null);
      } finally {
        setAuthChecked(true);
      }
    })();
  }, []);

  useEffect(() => {
    if (!selectedSecretProject) {
      setSecrets([]);
      return;
    }
    void refreshSecrets(selectedSecretProject).catch(setErrorMessage);
  }, [selectedSecretProject]);

  const onLogout = async () => {
    track("auth_logout");
    await apiRequest<null>("/auth/logout", { method: "POST" });
    location.reload();
  };

  const onCreateProject = async () => {
    try {
      const slug = projectSlug;
      await apiRequest<null>("/api/projects", {
        method: "POST",
        body: JSON.stringify({ slug, name: projectName }),
      });
      setProjectSlug("");
      setProjectName("");
      await refreshProjects();
      setSuccessMessage("Project created");
      track("project_created", { slug });
    } catch (error) {
      setErrorMessage(error);
    }
  };

  const onCreateKey = async () => {
    try {
      const project = projects.find(item => item.slug === selectedKeyProject);
      if (!project) {
        throw new Error("Choose a project before creating a key");
      }

      const created = await apiRequest<{ key: string }>("/api/keys", {
        method: "POST",
        body: JSON.stringify({
          name: keyName,
          scopes: [{ project_id: project.id, permission: keyPermission }],
        }),
      });
      setKeyOnce(created.key);
      setKeyName("");
      await refreshKeys();
      track("api_key_created", { project: selectedKeyProject });
    } catch (error) {
      setErrorMessage(error);
    }
  };

  const onRevokeKey = async (id: string) => {
    await apiRequest<null>(`/api/keys/${id}`, { method: "DELETE" });
    await refreshKeys();
  };

  const onRevealSecret = async (slug: string, key: string) => {
    const value = await apiRequest<SecretValue>("/api/secrets/read", {
      method: "POST",
      body: JSON.stringify({ project: slug, key }),
    });
    alert(`${value.key_name} = ${value.value}`);
  };

  const onDeleteSecret = async (slug: string, key: string) => {
    await apiRequest<null>("/api/secrets/delete", {
      method: "POST",
      body: JSON.stringify({ project: slug, key }),
    });
    await refreshSecrets(slug);
  };

  const onSaveSecret = async () => {
    try {
      if (!selectedSecretProject) {
        throw new Error("Choose a project before saving a secret");
      }
      await apiRequest<null>("/api/secrets/write", {
        method: "PUT",
        body: JSON.stringify({ project: selectedSecretProject, key: secretKey, value: secretValue }),
      });
      setSecretValue("");
      await refreshSecrets(selectedSecretProject);
      setSuccessMessage("Secret saved");
      track("secret_saved", { project: selectedSecretProject });
    } catch (error) {
      setErrorMessage(error);
    }
  };

  return (
    <main>
      <h1>ozzy secrets</h1>

      <AuthPromptCard authChecked={authChecked} me={me} />

      {me && (
        <>
          <AccountCard me={me} onLogout={onLogout} />

          <ProjectsSection
            projectSlug={projectSlug}
            projectName={projectName}
            projects={projects}
            onProjectSlugChange={setProjectSlug}
            onProjectNameChange={setProjectName}
            onCreateProject={onCreateProject}
          />

          <ApiKeysSection
            keyName={keyName}
            keyPermission={keyPermission}
            keyOnce={keyOnce}
            selectedKeyProject={selectedKeyProject}
            projects={projects}
            keys={keys}
            hasProjects={hasProjects}
            onKeyNameChange={setKeyName}
            onKeyPermissionChange={setKeyPermission}
            onSelectedProjectChange={setSelectedKeyProject}
            onCreateKey={onCreateKey}
            onRevokeKey={onRevokeKey}
          />

          <SecretsSection
            selectedSecretProject={selectedSecretProject}
            secretKey={secretKey}
            secretValue={secretValue}
            projects={projects}
            secrets={secrets}
            hasProjects={hasProjects}
            onSelectedProjectChange={setSelectedSecretProject}
            onSecretKeyChange={setSecretKey}
            onSecretValueChange={setSecretValue}
            onRevealSecret={onRevealSecret}
            onDeleteSecret={onDeleteSecret}
            onSaveSecret={onSaveSecret}
          />

          <StatusMessage message={message} />
        </>
      )}
    </main>
  );
}

export default App;
