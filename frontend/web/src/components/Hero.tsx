import { useState, useEffect, useCallback } from "react";
import { ArrowDown, ChevronLeft, ChevronRight, X } from "lucide-react";
import { useReleaseInfo } from "../hooks/useReleaseInfo";

const SCREENSHOTS = ["/SS-1.png", "/SS-2.png"];

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
    <section className="relative overflow-hidden bg-surface-dark pt-32 pb-20 md:pt-40 md:pb-28">
      <div className="hero-glow" />

      <div className="relative mx-auto max-w-6xl px-6">
        <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
          {/* Copy */}
          <div className="fade-in-up min-w-0 text-center lg:text-left">
            <p className="mb-4 inline-block rounded-full border border-accent/30 bg-accent/10 px-4 py-1.5 font-mono text-xs font-medium text-accent">
              {release ? `v${release.version}` : ""} — Free &amp; open-source
            </p>
            <h1 className="text-4xl leading-tight font-bold tracking-tight text-white md:text-5xl lg:text-5xl">
              Put your{" "}
              <span className="text-accent">AI subscriptions</span>
              {" "}to work as one quality pipeline.
            </h1>
            <p className="mt-6 max-w-lg text-lg leading-relaxed text-faint mx-auto lg:mx-0">
              You already have Claude, Codex, Copilot, Gemini, Kimi, and OpenCode.
              Maestro orchestrates them into one pipeline — planning in
              parallel, merging plans, coding, reviewing with multiple agents,
              and fixing — so you ship higher-quality code than any single
              agent could produce alone.
            </p>
            <div className="mt-8 flex flex-wrap items-center justify-center gap-4 lg:justify-start">
              <a
                href="#download"
                className="rounded-xl bg-white px-6 py-3 text-sm font-semibold text-surface-dark transition-colors duration-200 hover:bg-accent hover:text-white cursor-pointer"
              >
                Download for Free
              </a>
              <a
                href="#why"
                className="flex items-center gap-2 rounded-xl border border-border-dark px-6 py-3 text-sm font-medium text-faint transition-colors duration-200 hover:border-white hover:text-white cursor-pointer"
              >
                Why Multiple Agents?
                <ArrowDown className="h-4 w-4" />
              </a>
            </div>
          </div>

          {/* Screenshot carousel */}
          <div className="fade-in-up relative mx-auto w-full max-w-lg lg:mx-0" style={{ animationDelay: "0.2s" }}>
            <div className="overflow-hidden rounded-2xl border border-border-dark shadow-2xl shadow-black/40">
              <div
                className="relative aspect-[16/10] w-full cursor-pointer"
                onClick={() => setLightbox(true)}
              >
                {SCREENSHOTS.map((src, i) => (
                  <img
                    key={src}
                    src={src}
                    alt={`Maestro screenshot ${i + 1}`}
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
              className="absolute top-1/2 left-2 -translate-y-1/2 rounded-full border border-border-dark bg-surface-dark/80 p-1.5 text-faint backdrop-blur-sm transition-colors hover:text-white cursor-pointer"
            >
              <ChevronLeft className="h-4 w-4" />
            </button>
            <button
              onClick={next}
              aria-label="Next screenshot"
              className="absolute top-1/2 right-2 -translate-y-1/2 rounded-full border border-border-dark bg-surface-dark/80 p-1.5 text-faint backdrop-blur-sm transition-colors hover:text-white cursor-pointer"
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
                  className={`h-2 w-2 rounded-full transition-colors cursor-pointer ${
                    i === current ? "bg-accent" : "bg-border-dark"
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
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
          onClick={() => setLightbox(false)}
        >
          <button
            onClick={() => setLightbox(false)}
            aria-label="Close lightbox"
            className="absolute top-6 right-6 rounded-full border border-white/20 bg-white/10 p-2 text-white transition-colors hover:bg-white/20 cursor-pointer"
          >
            <X className="h-5 w-5" />
          </button>
          <img
            src={SCREENSHOTS[current]}
            alt={`Maestro screenshot ${current + 1}`}
            className="max-h-[90vh] max-w-[90vw] rounded-xl object-contain shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          />
        </div>
      )}
    </section>
  );
}
