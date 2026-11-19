import { useQuery } from "@tanstack/react-query";
import { CircleAlert } from "lucide-react";
import { Route, Routes } from "react-router";

import { Logo } from "./components/Logo";
import { Sidebar } from "./components/Sidebar";
import { api } from "./lib/api";
import { HomePage } from "./pages/HomePage";
import { SettingsPage } from "./pages/SettingsPage";

export function App() {
  // If the backend couldn't open the library, say why instead of showing a broken app.
  const startup = useQuery({ queryKey: ["startup-error"], queryFn: api.startupError, staleTime: Infinity });

  if (startup.isPending) return null;
  const fatal = startup.isError ? startup.error.message : startup.data;
  if (fatal) return <StartupFailure message={fatal} />;

  return (
    <div className="flex h-full">
      <Sidebar />
      <main className="min-w-0 flex-1 overflow-y-auto">
        <div className="px-12 pt-14 pb-10">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </div>
      </main>
    </div>
  );
}

function StartupFailure({ message }: { message: string }) {
  return (
    <div className="grid h-full place-items-center px-8">
      <div className="max-w-[34rem]">
        <Logo className="mb-6 size-12 opacity-80" />
        <h1 className="font-display text-[1.9rem] font-semibold tracking-[-0.02em]">Satchel couldn't open your library.</h1>
        <div className="mt-5 flex gap-3 rounded-xl bg-danger/8 px-4 py-3 text-[0.95rem]">
          <CircleAlert className="mt-0.5 size-[1.1rem] shrink-0 text-danger" aria-hidden />
          <p className="selectable">{message}</p>
        </div>
        <p className="mt-4 text-[0.9rem] text-chalk-mute">Fix the problem above, then close and reopen Satchel.</p>
      </div>
    </div>
  );
}
