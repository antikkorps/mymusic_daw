import { Link, useLocation } from "react-router";
import { cn } from "~/lib/utils";
import {
  LayoutDashboard,
  Sliders,
  Piano,
  Clock,
  Puzzle,
  Waves,
} from "lucide-react";

const navigation = [
  { name: "Dashboard", href: "/", icon: LayoutDashboard },
  { name: "Mixer", href: "/mixer", icon: Sliders },
  { name: "Piano Roll", href: "/piano-roll", icon: Piano },
  { name: "Timeline", href: "/timeline", icon: Clock },
  { name: "Plugins", href: "/plugins", icon: Puzzle },
  { name: "Effects", href: "/effects", icon: Waves },
];

export function Sidebar() {
  const location = useLocation();

  return (
    <aside className="w-64 bg-zinc-950 border-r border-zinc-800 flex flex-col">
      {/* Logo */}
      <div className="h-16 flex items-center px-6 border-b border-zinc-800">
        <h1 className="text-xl font-bold text-white">
          DAW <span className="text-cyan-500">Studio</span>
        </h1>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 py-4 space-y-1">
        {navigation.map((item) => {
          const isActive = location.pathname === item.href;
          const Icon = item.icon;

          return (
            <Link
              key={item.name}
              to={item.href}
              className={cn(
                "flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                isActive
                  ? "bg-cyan-500/10 text-cyan-400"
                  : "text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900"
              )}
            >
              <Icon className="w-5 h-5" />
              {item.name}
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-zinc-800">
        <div className="text-xs text-zinc-500">
          <div className="font-medium text-zinc-400 mb-1">Project</div>
          <div>Untitled Session</div>
          <div className="mt-2 text-zinc-600">v1.0.0-poc</div>
        </div>
      </div>
    </aside>
  );
}
