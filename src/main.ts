import { createApp } from "vue";
import { createPinia } from "pinia";
import "vditor/dist/index.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/vditor-override.css";
import App from "./App.vue";
import router from "./router";
import { useSession } from "./stores/session";

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);
app.use(router);

const session = useSession(pinia);

void session.load().finally(() => {
  app.mount("#app");
});