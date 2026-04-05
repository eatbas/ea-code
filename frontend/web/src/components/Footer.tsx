export function Footer() {
  const year = new Date().getFullYear();

  return (
    <footer className="border-t border-border-dark bg-surface-dark py-10">
      <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-6 md:flex-row">
        <div className="flex items-center gap-2.5">
          <img src="/app_logo.png" alt="Maestro logo" className="h-6 w-6 invert" />
          <span className="font-mono text-sm text-faint">
            &copy; {year} Maestro. Open-source under MIT.
          </span>
        </div>
        <div className="flex items-center gap-6">
          <a
            href="https://github.com/eatbas/maestro"
            className="text-sm text-faint transition-colors duration-200 hover:text-white cursor-pointer"
          >
            GitHub
          </a>
          <a
            href="#pipeline"
            className="text-sm text-faint transition-colors duration-200 hover:text-white cursor-pointer"
          >
            Docs
          </a>
          <a
            href="#features"
            className="text-sm text-faint transition-colors duration-200 hover:text-white cursor-pointer"
          >
            Features
          </a>
        </div>
      </div>
    </footer>
  );
}
