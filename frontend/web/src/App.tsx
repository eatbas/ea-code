import { Navbar } from "./components/Navbar";
import { Hero } from "./components/Hero";
import { AgentsBar } from "./components/AgentsBar";
import { Pipeline } from "./components/Pipeline";
import { Features } from "./components/Features";
import { CTA } from "./components/CTA";
import { Footer } from "./components/Footer";

export function App() {
  return (
    <div className="min-h-screen bg-surface text-white">
      <Navbar />
      <Hero />
      <AgentsBar />
      <Pipeline />
      <Features />
      <CTA />
      <Footer />
    </div>
  );
}
