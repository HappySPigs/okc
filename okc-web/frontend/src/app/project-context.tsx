import { createContext, useContext } from "react";
import type { ApiError } from "@/lib/api";
import type { ProjectStatusView, ProjectView } from "@/lib/types";

export interface ProjectCtx {
  projectId: string | null;
  project: ProjectView | null;
  status: ProjectStatusView | null;
  error: ApiError | null;
  loading: boolean;
  reload: () => void;
}

export const ProjectContext = createContext<ProjectCtx>({
  projectId: null, project: null, status: null, error: null, loading: false, reload: () => {},
});

export function useProjectContext(): ProjectCtx {
  return useContext(ProjectContext);
}

/** Extract the active project id from a `/projects/:id/...` path (null for /projects, /projects/new). */
export function projectIdFromPath(pathname: string): string | null {
  const m = pathname.match(/^\/projects\/([^/]+)/);
  if (!m) return null;
  const id = m[1];
  if (id === "new") return null;
  return id;
}
