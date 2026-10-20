import { HashRouter, Routes, Route } from "react-router-dom";
import ErrorBoundary from "./components/ErrorBoundary";
import { I18nProvider } from "./lib/i18n";
import Layout from "./components/Layout";
import Toast from "./components/Toast";
import Workspace from "./pages/Workspace";
import Projects from "./pages/Projects";
import ProjectDetail from "./pages/ProjectDetail";
import Requirements from "./pages/Requirements";
import RequirementDetail from "./pages/RequirementDetail";
import Board from "./pages/Board";
import TaskDetail from "./pages/TaskDetail";
import SkillManagement from "./pages/SkillManagement";
import SkillEvolution from "./pages/SkillEvolution";
import Agents from "./pages/Agents";
import Sessions from "./pages/Sessions";
import SessionDetail from "./pages/SessionDetail";
import Commits from "./pages/Commits";
import Settings from "./pages/Settings";
import Cost from "./pages/Cost";
import Statistics from "./pages/Statistics";
import AuditLog from "./pages/AuditLog";
import ImportExport from "./pages/ImportExport";
import SystemMonitor from "./pages/SystemMonitor";

function App() {
  return (
    <I18nProvider>
      <HashRouter>
        <ErrorBoundary>
          <Routes>
            <Route path="/" element={<Layout />}>
              <Route index element={<Workspace />} />
              <Route path="projects" element={<Projects />} />
              <Route path="projects/:id" element={<ProjectDetail />} />
              <Route path="requirements" element={<Requirements />} />
              <Route path="requirements/:id" element={<RequirementDetail />} />
              <Route path="board" element={<Board />} />
              <Route path="tasks/:id" element={<TaskDetail />} />
              <Route path="skills" element={<SkillManagement />} />
              <Route path="skills/:id/evolution" element={<SkillEvolution />} />
              <Route path="agents" element={<Agents />} />
              <Route path="sessions" element={<Sessions />} />
              <Route path="sessions/:id" element={<SessionDetail />} />
              <Route path="commits" element={<Commits />} />
              <Route path="cost" element={<Cost />} />
              <Route path="statistics" element={<Statistics />} />
              <Route path="audit-log" element={<AuditLog />} />
              <Route path="import-export" element={<ImportExport />} />
              <Route path="system-monitor" element={<SystemMonitor />} />
              <Route path="settings" element={<Settings />} />
            </Route>
          </Routes>
          <Toast />
        </ErrorBoundary>
      </HashRouter>
    </I18nProvider>
  );
}

export default App;
