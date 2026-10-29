import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { PairScreen } from "./components/PairScreen";
import { RecordScreen } from "./components/RecordScreen";
import { SetupScreen } from "./components/SetupScreen";
import { Settings } from "./settings/Settings";
import "./index.css";

// One bundle, five kinds of window, picked by the URL hash: the panels (touch-driven), settings
// (`#settings[/<page>]`, mouse-driven), pairing (`#pair/<name>`, shown until a finger lands), setup
// (`#setup/<radius>`, one per monitor while setup runs) and the report's recording prompt
// (`#record/<seconds>`).
const hash = window.location.hash;
const settings = hash === "#settings" || hash.startsWith("#settings/");
if (settings) document.documentElement.classList.add("settings");
const pair = hash.startsWith("#pair/") ? decodeURIComponent(hash.slice("#pair/".length)) : null;
const setup = hash.startsWith("#setup/") ? Number(hash.slice("#setup/".length)) : null;
const record = hash.startsWith("#record/") ? Number(hash.slice("#record/".length)) : null;

const page = settings ? (
  <Settings />
) : pair !== null ? (
  <PairScreen name={pair} />
) : setup !== null ? (
  <SetupScreen radius={setup} />
) : record !== null ? (
  <RecordScreen seconds={record} />
) : (
  <App />
);
createRoot(document.getElementById("root")!).render(<StrictMode>{page}</StrictMode>);
