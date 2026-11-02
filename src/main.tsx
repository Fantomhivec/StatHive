import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { I18nProvider } from "./i18n";
import { applyTheme, readTheme } from "./theme";

applyTheme(readTheme());

// Desktop shell: no Chromium/WebView default context menu on right-click.
document.addEventListener("contextmenu", (e) => e.preventDefault());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <I18nProvider>
        <App />
      </I18nProvider>
    </BrowserRouter>
  </React.StrictMode>,
);
