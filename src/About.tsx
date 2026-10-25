import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "./api";
import Logo from "./Logo";

const LINKS = [
  { label: "Website", url: "https://itsazaira.com" },
  { label: "GitHub", url: "https://github.com/itsazaira/clipstash" },
  { label: "Instagram", url: "https://www.instagram.com/itsazaira/" },
  { label: "LinkedIn", url: "https://www.linkedin.com/company/itsazaira" },
  { label: "Support", url: "mailto:hello@itsazaira.com" },
];

export default function About() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && window.close();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <main className="about">
      <Logo size={72} />
      <h1>ClipStash</h1>
      <p className="tagline">Your last 50 copies, one shortcut away</p>
      {version && <p className="version">Version {version}</p>}

      <div className="maker">
        <div className="maker-name">by Its Azaira</div>
        <div className="maker-tagline">Ideas, built from A to Z</div>
      </div>

      <nav className="links" aria-label="Its Azaira links">
        {LINKS.map((link) => (
          <button key={link.label} className="link" onClick={() => api.openLink(link.url)}>
            {link.label}
          </button>
        ))}
      </nav>

      <p className="licence">Free forever. Open source, MIT licence.</p>
    </main>
  );
}
