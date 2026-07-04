import type { Project } from "./types";

type ProjectsSectionProps = {
  projectSlug: string;
  projectName: string;
  projects: Project[];
  onProjectSlugChange: (value: string) => void;
  onProjectNameChange: (value: string) => void;
  onCreateProject: () => void | Promise<void>;
};

export function ProjectsSection({
  projectSlug,
  projectName,
  projects,
  onProjectSlugChange,
  onProjectNameChange,
  onCreateProject,
}: ProjectsSectionProps) {
  return (
    <>
      <h2>Projects</h2>
      <div className="card">
        <div className="row">
          <div>
            <label>
              Slug
              <input value={projectSlug} onChange={event => onProjectSlugChange(event.target.value)} placeholder="my-app" />
            </label>
          </div>
          <div>
            <label>
              Name
              <input value={projectName} onChange={event => onProjectNameChange(event.target.value)} placeholder="My App" />
            </label>
          </div>
        </div>
        <button type="button" onClick={() => void onCreateProject()}>
          Create project
        </button>
        <ul>
          {projects.map(project => (
            <li key={project.id}>
              <span>
                <code>{project.slug}</code> {project.name}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}
