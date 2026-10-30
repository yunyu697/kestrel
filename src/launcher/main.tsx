import React from "react";
import { createRoot } from "react-dom/client";
import "../shared/theme.css";
import "./launcher.css";
import Launcher from "./Launcher";

const el = document.getElementById("root");
if (el) {
  createRoot(el).render(
    <React.StrictMode>
      <Launcher />
    </React.StrictMode>
  );
}
