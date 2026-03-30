export function Footer() {
  const year = new Date().getFullYear();

  return (
    <footer className="border-t border-border py-10">
      <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-6 md:flex-row">
        <span className="font-mono text-sm text-muted">
          &copy; {year} Maestro. Open-source under MIT.
        </span>
        <div className="flex items-center gap-6">
          <a
            href="https://github.com/eatbas/maestro"
            className="text-sm text-muted transition-colors duration-200 hover:text-white cursor-pointer"
          >
            GitHub
          </a>
          <a
            href="#pipeline"
            className="text-sm text-muted transition-colors duration-200 hover:text-white cursor-pointer"
          >
            Docs
          </a>
          <a
            href="#features"
            className="text-sm text-muted transition-colors duration-200 hover:text-white cursor-pointer"
          >
            Features
          </a>
        </div>
      </div>
    </footer>
  );
}
