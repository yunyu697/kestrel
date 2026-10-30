import React from "react";
import { createRoot } from "react-dom/client";
import "../shared/theme.css";
import "./settings.css";
import Settings from "./Settings";

const el = document.getElementById("root");
if (el) {
  createRoot(el).render(
    <React.StrictMode>
      <Settings />
    </React.StrictMode>
  );
}
