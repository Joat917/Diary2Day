import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/today" },
    {
      path: "/today",
      name: "today",
      component: () => import("./views/TodayView.vue"),
    },
    {
      path: "/discover",
      name: "discover",
      component: () => import("./views/DiscoverView.vue"),
    },
    {
      path: "/me",
      name: "me",
      component: () => import("./views/MeView.vue"),
    },
    {
      path: "/j/:id",
      name: "detail",
      component: () => import("./views/DetailView.vue"),
    },
    { path: "/:pathMatch(.*)*", redirect: "/today" },
  ],
});

export default router;