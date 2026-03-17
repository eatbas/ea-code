import { useState } from "react";
import { Menu, X } from "lucide-react";

const NAV_LINKS = [
  { label: "Why", href: "#why" },
  { label: "Pipeline", href: "#pipeline" },
  { label: "Features", href: "#features" },
];

export function Navbar() {
  const [open, setOpen] = useState(false);

  return (
    <nav className="fixed top-4 left-4 right-4 z-50 mx-auto max-w-6xl rounded-2xl border border-border bg-surface/80 backdrop-blur-xl">
      <div className="flex items-center justify-between px-6 py-3">
        {/* Logo */}
        <a href="#" className="flex items-center gap-2 font-mono text-lg font-bold tracking-tight">
          <img src="/logo.png" alt="EA Code logo" className="h-14 w-14" />
          <span>EA Code</span>
        </a>

        {/* Desktop links */}
        <ul className="hidden items-center gap-8 md:flex">
          {NAV_LINKS.map((link) => (
            <li key={link.href}>
              <a
                href={link.href}
                className="text-sm text-muted transition-colors duration-200 hover:text-white"
              >
                {link.label}
              </a>
            </li>
          ))}
        </ul>

        {/* CTA + mobile toggle */}
        <div className="flex items-center gap-3">
          <a
            href="#download"
            className="hidden rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-surface transition-colors duration-200 hover:bg-accent-hover cursor-pointer md:inline-block"
          >
            Download
          </a>
          <button
            onClick={() => setOpen(!open)}
            className="inline-flex items-center justify-center rounded-lg p-2 text-muted transition-colors hover:text-white md:hidden cursor-pointer"
            aria-label={open ? "Close menu" : "Open menu"}
          >
            {open ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
          </button>
        </div>
      </div>

      {/* Mobile menu */}
      {open && (
        <div className="border-t border-border px-6 pb-4 pt-2 md:hidden">
          <ul className="flex flex-col gap-3">
            {NAV_LINKS.map((link) => (
              <li key={link.href}>
                <a
                  href={link.href}
                  onClick={() => setOpen(false)}
                  className="block text-sm text-muted transition-colors hover:text-white"
                >
                  {link.label}
                </a>
              </li>
            ))}
            <li>
              <a
                href="#download"
                onClick={() => setOpen(false)}
                className="mt-1 block rounded-lg bg-accent px-4 py-2 text-center text-sm font-semibold text-surface"
              >
                Download
              </a>
            </li>
          </ul>
        </div>
      )}
    </nav>
  );
}
