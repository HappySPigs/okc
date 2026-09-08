import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./app/AppShell";
import { RequireAdmin } from "./app/guards";
import { Login } from "./screens/Login";
import { Forbidden } from "./screens/Forbidden";
import { NotFound } from "./screens/NotFound";
import { UploadPortal } from "./screens/upload/UploadPortal";
import { UploadPage } from "./screens/upload/UploadPage";
import { UploadDone } from "./screens/upload/UploadDone";
import { ProjectsList } from "./screens/projects/ProjectsList";
import { Overview } from "./screens/projects/Overview";
import { Sources } from "./screens/projects/Sources";
import { Tokens } from "./screens/projects/Tokens";
import { Integration } from "./screens/projects/Integration";
import { ReviewGate } from "./screens/review/ReviewGate";
import { Taxonomy } from "./screens/review/Taxonomy";
import { ClusterReview } from "./screens/review/ClusterReview";
import { Regenerate } from "./screens/review/Regenerate";
import { Compiled } from "./screens/serving/Compiled";
import { Provenance } from "./screens/serving/Provenance";
import { Serving } from "./screens/serving/Serving";
import { Contract } from "./screens/serving/Contract";
import { Users } from "./screens/settings/Users";
import { Account } from "./screens/settings/Account";

export function App() {
  return (
    <Routes>
      {/* Auth + contributor upload shell (no app shell) */}
      <Route path="/login" element={<Login />} />
      <Route path="/403" element={<Forbidden />} />
      {/* Contributor upload shell. NOTE: the SPA portal lives under /upload/* —
          the backend owns GET /u/{token} as a JSON API (upload_url=/u/{token}),
          so the human portal cannot share that path. API calls still target /u/. */}
      <Route path="/upload/:token" element={<UploadPortal />} />
      <Route path="/upload/:token/file" element={<UploadPage />} />
      <Route path="/upload/:token/done" element={<UploadDone />} />

      {/* Admin app shell */}
      <Route element={<RequireAdmin><AppShell /></RequireAdmin>}>
        <Route index element={<Navigate to="/projects" replace />} />
        <Route path="/projects" element={<ProjectsList />} />
        <Route path="/projects/new" element={<ProjectsList />} />
        <Route path="/settings/users" element={<Users />} />
        <Route path="/settings/account" element={<Account />} />
        <Route path="/projects/:id" element={<Overview />} />
        <Route path="/projects/:id/sources" element={<Sources />} />
        <Route path="/projects/:id/tokens" element={<Tokens />} />
        <Route path="/projects/:id/tokens/new" element={<Tokens />} />
        <Route path="/projects/:id/integration" element={<Integration />} />
        <Route path="/projects/:id/review" element={<ReviewGate />} />
        <Route path="/projects/:id/review/taxonomy" element={<Taxonomy />} />
        <Route path="/projects/:id/review/clusters/:clusterId" element={<ClusterReview />} />
        <Route path="/projects/:id/review/clusters/:clusterId/regenerate" element={<Regenerate />} />
        <Route path="/projects/:id/compiled" element={<Compiled />} />
        <Route path="/projects/:id/serving" element={<Serving />} />
        <Route path="/projects/:id/serving/verify" element={<Provenance />} />
        <Route path="/projects/:id/serving/contract" element={<Contract />} />
      </Route>

      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
