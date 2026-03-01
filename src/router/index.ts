import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "analysis",
      component: () => import("@/views/AnalysisView.vue"),
    },
    {
      path: "/projects",
      name: "projects",
      component: () => import("@/views/ProjectView.vue"),
    },
    {
      path: "/editor",
      name: "editor",
      component: () => import("@/views/EditorView.vue"),
    },
    {
      path: "/graph",
      name: "graph",
      component: () => import("@/views/GraphView.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("@/views/SettingsView.vue"),
    },
  ],
});

export default router;
