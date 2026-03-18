import { useState, useEffect, useCallback } from "react";
import { ArrowDown, ChevronLeft, ChevronRight, X } from "lucide-react";
import { useReleaseInfo } from "../hooks/useReleaseInfo";

const SCREENSHOTS = ["/ss1.png", "/ss2.png"];

export function Hero() {
  const { release } = useReleaseInfo();
  const [current, setCurrent] = useState(0);
  const [lightbox, setLightbox] = useState(false);

  const next = useCallback(() => setCurrent((i) => (i + 1) % SCREENSHOTS.length), []);
  const prev = useCallback(() => setCurrent((i) => (i - 1 + SCREENSHOTS.length) % SCREENSHOTS.length), []);

  useEffect(() => {
    const id = setInterval(next, 5000);
    return () => clearInterval(id);
  }, [next]);

  return (
    <section className="relative overflow-hidden pt-32 pb-20 md:pt-40 md:pb-28">
      <div className="hero-glow" />

      <div className="relative mx-auto max-w-6xl px-6">
        <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
          {/* Copy */}
          <div className="fade-in-up text-center lg:text-left">
            <p className="mb-4 inline-block rounded-full border border-accent/30 bg-accent/10 px-4 py-1.5 font-mono text-xs font-medium text-accent">
              {release ? `v${release.version}` : ""} — Free &amp; open-source
            </p>
            <h1 className="font-mono text-4xl leading-tight font-bold tracking-tight md:text-5xl lg:text-6xl">
              Put your <span className="text-accent">AI subscriptions to work</span>
              <br />
              as one quality pipeline.
            </h1>
            <p className="mt-6 max-w-lg text-lg leading-relaxed text-muted md:text-xl mx-auto lg:mx-0">
              You already have Claude, Codex, Gemini, Kimi, and OpenCode.
              EA Code orchestrates those subscriptions into one pipeline, so
              each model can plan, code, and review where it performs best,
              helping you ship higher-quality code than a single agent alone.
            </p>
            <div className="mt-8 flex flex-wrap items-center justify-center gap-4 lg:justify-start">
              <a
                href="#download"
                className="rounded-xl bg-accent px-6 py-3 text-sm font-semibold text-surface transition-colors duration-200 hover:bg-accent-hover cursor-pointer"
              >
                Download for Free
              </a>
              <a
                href="#why"
                className="flex items-center gap-2 rounded-xl border border-border px-6 py-3 text-sm font-medium text-muted transition-colors duration-200 hover:border-muted hover:text-white cursor-pointer"
              >
                Why Multiple Agents?
                <ArrowDown className="h-4 w-4" />
              </a>
            </div>
          </div>

          {/* Screenshot carousel */}
          <div className="fade-in-up relative mx-auto w-full max-w-lg lg:mx-0" style={{ animationDelay: "0.2s" }}>
            <div className="overflow-hidden rounded-2xl border border-border bg-surface-elevated shadow-2xl shadow-black/40">
              <div
                className="relative aspect-[16/10] w-full cursor-pointer"
                onClick={() => setLightbox(true)}
              >
                {SCREENSHOTS.map((src, i) => (
                  <img
                    key={src}
                    src={src}
                    alt={`ea-code screenshot ${i + 1}`}
                    className={`absolute inset-0 h-full w-full object-cover transition-opacity duration-500 ${
                      i === current ? "opacity-100" : "opacity-0"
                    }`}
                  />
                ))}
              </div>
            </div>

            {/* Controls */}
            <button
              onClick={prev}
              aria-label="Previous screenshot"
              className="absolute top-1/2 left-2 -translate-y-1/2 rounded-full border border-border bg-surface/80 p-1.5 text-muted backdrop-blur-sm transition-colors hover:text-white"
            >
              <ChevronLeft className="h-4 w-4" />
            </button>
            <button
              onClick={next}
              aria-label="Next screenshot"
              className="absolute top-1/2 right-2 -translate-y-1/2 rounded-full border border-border bg-surface/80 p-1.5 text-muted backdrop-blur-sm transition-colors hover:text-white"
            >
              <ChevronRight className="h-4 w-4" />
            </button>

            {/* Dots */}
            <div className="mt-4 flex justify-center gap-2">
              {SCREENSHOTS.map((_, i) => (
                <button
                  key={i}
                  onClick={() => setCurrent(i)}
                  aria-label={`Go to screenshot ${i + 1}`}
                  className={`h-2 w-2 rounded-full transition-colors ${
                    i === current ? "bg-accent" : "bg-border"
                  }`}
                />
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Lightbox */}
      {lightbox && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
          onClick={() => setLightbox(false)}
        >
          <button
            onClick={() => setLightbox(false)}
            aria-label="Close lightbox"
            className="absolute top-6 right-6 rounded-full border border-white/20 bg-white/10 p-2 text-white transition-colors hover:bg-white/20"
          >
            <X className="h-5 w-5" />
          </button>
          <img
            src={SCREENSHOTS[current]}
            alt={`ea-code screenshot ${current + 1}`}
            className="max-h-[90vh] max-w-[90vw] rounded-xl object-contain shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          />
        </div>
      )}
    </section>
  );
}
