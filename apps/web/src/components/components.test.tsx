import { describe, expect, mock, test } from "bun:test";
import { fireEvent, render, within } from "@testing-library/react";
import { AccountCard } from "./AccountCard";
import { ApiKeysSection } from "./ApiKeysSection";
import { AuthPromptCard } from "./AuthPromptCard";
import { ProjectsSection } from "./ProjectsSection";
import { SecretsSection } from "./SecretsSection";
import { StatusMessage } from "./StatusMessage";

const projects = [
  { id: "proj-1", slug: "alpha", name: "Alpha" },
  { id: "proj-2", slug: "beta", name: "Beta" },
];

describe("AuthPromptCard", () => {
  test("renders sign-in prompt only when auth is checked and user is signed out", () => {
    const { container, rerender } = render(<AuthPromptCard authChecked={false} me={null} />);
    expect(within(container).queryByText("Sign in with GitHub")).toBeNull();

    rerender(<AuthPromptCard authChecked={true} me={{ login: "willy", name: "Willy" }} />);
    expect(within(container).queryByText("Sign in with GitHub")).toBeNull();

    rerender(<AuthPromptCard authChecked={true} me={null} />);
    const queries = within(container);
    const signInButton = queries.getByRole("button", { name: "Sign in with GitHub" });
    expect(signInButton).toBeTruthy();
    expect(signInButton.closest("a")?.getAttribute("href")).toBe("/auth/github");
  });
});

describe("AccountCard", () => {
  test("renders account info and calls logout handler", () => {
    const onLogout = mock(() => {});
    const { container } = render(<AccountCard me={{ login: "willy", name: "William" }} onLogout={onLogout} />);
    const queries = within(container);

    expect(queries.getByText("willy")).toBeTruthy();
    expect(queries.getByText("William")).toBeTruthy();

    fireEvent.click(queries.getByRole("button", { name: "Sign out" }));
    expect(onLogout).toHaveBeenCalledTimes(1);
  });
});

describe("ProjectsSection", () => {
  test("renders projects and wires create callback", () => {
    const onProjectSlugChange = mock((_value: string) => {});
    const onProjectNameChange = mock((_value: string) => {});
    const onCreateProject = mock(() => {});

    const { container } = render(
      <ProjectsSection
        projectSlug="alpha"
        projectName="Alpha"
        projects={projects}
        onProjectSlugChange={onProjectSlugChange}
        onProjectNameChange={onProjectNameChange}
        onCreateProject={onCreateProject}
      />,
    );
    const queries = within(container);

    expect(queries.getByRole("heading", { name: "Projects" })).toBeTruthy();
    expect(queries.getByText("Alpha", { selector: "span" })).toBeTruthy();

    expect((queries.getByPlaceholderText("my-app") as HTMLInputElement).value).toBe("alpha");
    expect((queries.getByPlaceholderText("My App") as HTMLInputElement).value).toBe("Alpha");

    fireEvent.click(queries.getByRole("button", { name: "Create project" }));
    expect(onCreateProject).toHaveBeenCalledTimes(1);
  });
});

describe("ApiKeysSection", () => {
  test("renders keys and wires callbacks", () => {
    const onKeyNameChange = mock((_value: string) => {});
    const onKeyPermissionChange = mock((_value: string) => {});
    const onSelectedProjectChange = mock((_value: string) => {});
    const onCreateKey = mock(() => {});
    const onRevokeKey = mock((_id: string) => {});

    const { container } = render(
      <ApiKeysSection
        keyName="deploy"
        keyPermission="read"
        keyOnce="oz_live_123"
        selectedKeyProject="alpha"
        projects={projects}
        keys={[{ id: "key-1", key_prefix: "oz_pre", name: "Deploy key" }]}
        hasProjects={true}
        onKeyNameChange={onKeyNameChange}
        onKeyPermissionChange={onKeyPermissionChange}
        onSelectedProjectChange={onSelectedProjectChange}
        onCreateKey={onCreateKey}
        onRevokeKey={onRevokeKey}
      />,
    );
    const queries = within(container);

    expect((queries.getByPlaceholderText("CI deploy") as HTMLInputElement).value).toBe("deploy");

    fireEvent.change(queries.getByLabelText("Project"), { target: { value: "beta" } });
    expect(onSelectedProjectChange).toHaveBeenCalledWith("beta");

    fireEvent.change(queries.getByLabelText("Permission"), { target: { value: "write" } });
    expect(onKeyPermissionChange).toHaveBeenCalledWith("write");

    fireEvent.click(queries.getByRole("button", { name: "Create API key" }));
    expect(onCreateKey).toHaveBeenCalledTimes(1);

    expect(queries.getByText("Copy this key now. It won't be shown again.")).toBeTruthy();
    expect(queries.getByText("oz_live_123")).toBeTruthy();

    fireEvent.click(queries.getByRole("button", { name: "Revoke" }));
    expect(onRevokeKey).toHaveBeenCalledWith("key-1");
  });

  test("disables project controls when there are no projects", () => {
    const { container } = render(
      <ApiKeysSection
        keyName=""
        keyPermission="read"
        keyOnce={null}
        selectedKeyProject=""
        projects={[]}
        keys={[]}
        hasProjects={false}
        onKeyNameChange={() => {}}
        onKeyPermissionChange={() => {}}
        onSelectedProjectChange={() => {}}
        onCreateKey={() => {}}
        onRevokeKey={() => {}}
      />,
    );
    const queries = within(container);

    expect(queries.getByLabelText("Project").hasAttribute("disabled")).toBe(true);
    expect(queries.getByRole("button", { name: "Create API key" }).hasAttribute("disabled")).toBe(true);
  });
});

describe("SecretsSection", () => {
  test("renders secrets and wires action callbacks", () => {
    const onSelectedProjectChange = mock((_value: string) => {});
    const onSecretKeyChange = mock((_value: string) => {});
    const onSecretValueChange = mock((_value: string) => {});
    const onRevealSecret = mock((_slug: string, _key: string) => {});
    const onDeleteSecret = mock((_slug: string, _key: string) => {});
    const onSaveSecret = mock(() => {});

    const { container } = render(
      <SecretsSection
        selectedSecretProject="alpha"
        secretKey="DATABASE_URL"
        secretValue="postgres://"
        projects={projects}
        secrets={[{ key_name: "DATABASE_URL", version: 3 }]}
        hasProjects={true}
        onSelectedProjectChange={onSelectedProjectChange}
        onSecretKeyChange={onSecretKeyChange}
        onSecretValueChange={onSecretValueChange}
        onRevealSecret={onRevealSecret}
        onDeleteSecret={onDeleteSecret}
        onSaveSecret={onSaveSecret}
      />,
    );
    const queries = within(container);

    fireEvent.change(queries.getByLabelText("Project"), { target: { value: "beta" } });
    expect(onSelectedProjectChange).toHaveBeenCalledWith("beta");

    expect((queries.getByPlaceholderText("DATABASE_URL") as HTMLInputElement).value).toBe("DATABASE_URL");
    expect((queries.getByPlaceholderText("value") as HTMLInputElement).value).toBe("postgres://");

    fireEvent.click(queries.getByRole("button", { name: "Reveal" }));
    expect(onRevealSecret).toHaveBeenCalledWith("alpha", "DATABASE_URL");

    fireEvent.click(queries.getByRole("button", { name: "Delete" }));
    expect(onDeleteSecret).toHaveBeenCalledWith("alpha", "DATABASE_URL");

    fireEvent.click(queries.getByRole("button", { name: "Save secret" }));
    expect(onSaveSecret).toHaveBeenCalledTimes(1);
  });

  test("disables project controls when there are no projects", () => {
    const { container } = render(
      <SecretsSection
        selectedSecretProject=""
        secretKey=""
        secretValue=""
        projects={[]}
        secrets={[]}
        hasProjects={false}
        onSelectedProjectChange={() => {}}
        onSecretKeyChange={() => {}}
        onSecretValueChange={() => {}}
        onRevealSecret={() => {}}
        onDeleteSecret={() => {}}
        onSaveSecret={() => {}}
      />,
    );
    const queries = within(container);

    expect(queries.getByLabelText("Project").hasAttribute("disabled")).toBe(true);
    expect(queries.getByRole("button", { name: "Save secret" }).hasAttribute("disabled")).toBe(true);
  });
});

describe("StatusMessage", () => {
  test("renders muted and error variants", () => {
    const { container, rerender } = render(<StatusMessage message={{ text: "Saved", isError: false }} />);
    expect(within(container).getByText("Saved").className).toBe("muted");

    rerender(<StatusMessage message={{ text: "Failed", isError: true }} />);
    expect(within(container).getByText("Failed").className).toBe("error");
  });
});
