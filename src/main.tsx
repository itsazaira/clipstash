import React from "react";
import ReactDOM from "react-dom/client";
import Popup from "./Popup";
import About from "./About";
import "./styles.css";

// One bundle, two windows: the popup (default) and About (#about).
const isAbout = window.location.hash === "#about";
document.body.classList.add(isAbout ? "about-body" : "popup-body");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isAbout ? <About /> : <Popup />}</React.StrictMode>,
);
