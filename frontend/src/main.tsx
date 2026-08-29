import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { AppLayout } from "@/app/AppLayout";
import { DashboardPage } from "@/pages/Dashboard/DashboardPage";
import { ProjectsPage } from "@/pages/Projects/ProjectsPage";
import { ProjectDetailPage } from "@/pages/Project/ProjectDetailPage";
import { TasksPage } from "@/pages/Tasks/TasksPage";
import { CalendarPage } from "@/pages/Calendar/CalendarPage";
import { SettingsPage } from "@/pages/Settings/SettingsPage";
import "@/styles/global.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route index element={<DashboardPage />} />
          <Route path="projects" element={<ProjectsPage />} />
          <Route path="projects/:id" element={<ProjectDetailPage />} />
          <Route path="tasks" element={<TasksPage />} />
          <Route path="calendar" element={<CalendarPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </StrictMode>,
);
