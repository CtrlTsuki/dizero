import "@fontsource-variable/inter";
import "@fontsource-variable/geist-mono";
import "./styles/app.css";
import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("#app root missing");

export default mount(App, { target });
