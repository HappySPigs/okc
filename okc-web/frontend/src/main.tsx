import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { AuthProvider } from "./app/auth";
import { Toaster } from "./components/ui/toaster";
import { App } from "./App";
// Radix color scales (light) — loaded via Vite's CSS pipeline (not Tailwind's).
import "@radix-ui/colors/slate.css";
import "@radix-ui/colors/indigo.css";
import "@radix-ui/colors/red.css";
import "@radix-ui/colors/amber.css";
import "@radix-ui/colors/grass.css";
import "@radix-ui/colors/violet.css";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <App />
        <Toaster />
      </AuthProvider>
    </BrowserRouter>
  </StrictMode>,
);
