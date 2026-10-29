import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { PairScreen } from "./components/PairScreen";
import { Settings } from "./settings/Settings";
import "./index.css";

// One bundle, three kinds of window, picked by the URL hash: the panels (touch-driven), settings
// (`#settings[/<page>]`, mouse-driven) and pairing (`#pair/<name>`, shown until a finger lands).
const hash = window.location.hash;
const settings = hash === "#settings" || hash.startsWith("#settings/");
if (settings) document.documentElement.classList.add("settings");
const pair = hash.startsWith("#pair/") ? decodeURIComponent(hash.slice("#pair/".length)) : null;

createRoot(document.getElementById("root")!).render(
  <StrictMode>{settings ? <Settings /> : pair !== null ? <PairScreen name={pair} /> : <App />}</StrictMode>,
);
