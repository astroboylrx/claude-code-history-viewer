import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "overlayscrollbars/overlayscrollbars.css";
import "./index.css";
import "./scrollbar.css";
import App from "./App.tsx";
import { ErrorBoundary } from "./components/ErrorBoundary.tsx";
import "./i18n";
import { PlatformProvider } from "./contexts/platform";
import { ThemeProvider } from "./contexts/theme/ThemeProvider.tsx";
import { ModalProvider } from "./contexts/modal/ModalProvider.tsx";
import { Toaster } from "sonner";
import { initAuthToken, recoverAuthFromErrorQuery } from "./utils/platform";

// No-op in Tauri desktop mode.)
initAuthToken();
// If startup hit `?auth_error=1`, prompt for token and reload once recovered.
recoverAuthFromErrorQuery();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <PlatformProvider>
        <ThemeProvider>
          <ModalProvider>
            <App />
            <Toaster />
          </ModalProvider>
        </ThemeProvider>
      </PlatformProvider>
    </ErrorBoundary>
  </StrictMode>
);
