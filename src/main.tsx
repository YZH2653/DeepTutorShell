import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// 屏蔽 WebView2 右键菜单（刷新/另存为/打印）
document.addEventListener("contextmenu", (e) => e.preventDefault());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
