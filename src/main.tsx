import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/manrope";
import Popup from "./Popup";
import { applyTheme, loadTheme } from "./theme";
import "./styles.css";

// Set the theme before the first paint so the popup never flashes the wrong colours.
applyTheme(loadTheme());

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Popup />
  </React.StrictMode>,
);
