import { createRouter, createWebHistory } from "vue-router";

/**
 * 路由已简化为最小配置。
 * 核心 IDE 功能（编辑器、文件树、AI 面板、终端）全部内嵌在 MainLayout 中，
 * 无需路由切换。保留 router 实例以备将来扩展（如弹出分析窗口等）。
 */
const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: () => import("@/App.vue"),
    },
  ],
});

export default router;
