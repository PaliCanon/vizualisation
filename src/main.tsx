import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "./i18n";
import "./styles.css";
import { App } from "./App";
import { Footer } from "./components/Footer";
import { NotFoundPage, Page, pageFromPath } from "./components/Pages";

// The map (/), the site pages (/acknowledgements, /terms…) and a 404 for anything else, switched in place
// like palicanon.app does (history.pushState + popstate).
function Root() {
  const [path, setPath] = useState(location.pathname);
  useEffect(() => {
    const on = () => setPath(location.pathname);
    window.addEventListener("popstate", on);
    return () => window.removeEventListener("popstate", on);
  }, []);
  const kind = pageFromPath(path);
  if (!kind && path === "/") return <App />;
  if (!kind) return <NotFoundPage />;
  const home = () => {
    history.pushState(null, "", "/");
    window.dispatchEvent(new PopStateEvent("popstate"));
  };
  return (
    <div className="flex min-h-dvh flex-col">
      <div className="flex-1"><Page kind={kind} onBack={home} /></div>
      <Footer />
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
