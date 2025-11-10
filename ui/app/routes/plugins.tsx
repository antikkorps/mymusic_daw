import { useState, useEffect } from "react";
import { Layout } from "~/components/layout/Layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Skeleton } from "~/components/ui/skeleton";
import { mockPlugins } from "~/lib/mockData";
import type { Plugin, PluginCategory } from "~/types/plugin";
import { cn } from "~/lib/utils";
import { useToast } from "~/lib/toast";
import { Puzzle, Download, Power, Search } from "lucide-react";

const CATEGORIES: (PluginCategory | "All")[] = [
  "All",
  "Instrument",
  "Effect",
  "Dynamics",
  "EQ",
  "Filter",
  "Delay",
  "Reverb",
  "Distortion",
];

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<PluginCategory | "All">("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const { showToast } = useToast();

  // Simulate loading plugins
  useEffect(() => {
    const timer = setTimeout(() => {
      setPlugins(mockPlugins);
      setIsLoading(false);
    }, 1200); // 1.2 second loading time

    return () => clearTimeout(timer);
  }, []);

  const togglePluginLoad = (pluginId: string) => {
    const plugin = plugins.find((p) => p.id === pluginId);
    if (!plugin) return;

    const isLoading = !plugin.loaded;

    setPlugins((prev) =>
      prev.map((p) =>
        p.id === pluginId
          ? { ...p, loaded: isLoading, instanceId: isLoading ? `inst-${Date.now()}` : undefined }
          : p
      )
    );

    showToast(
      isLoading ? `Plugin "${plugin.name}" loaded successfully` : `Plugin "${plugin.name}" unloaded`,
      isLoading ? "success" : "info"
    );
  };

  const filteredPlugins = plugins.filter((plugin) => {
    const matchesCategory =
      selectedCategory === "All" || plugin.category === selectedCategory;
    const matchesSearch =
      searchQuery === "" ||
      plugin.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      plugin.vendor.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const loadedPlugins = plugins.filter((p) => p.loaded);

  return (
    <Layout>
      <div className="h-full bg-zinc-900 p-6 overflow-auto">
        <div className="max-w-7xl mx-auto space-y-6">
          {/* Header */}
          <div>
            <h2 className="text-2xl font-bold text-white">Plugin Manager</h2>
            <p className="text-zinc-400 mt-1">
              Manage your CLAP, VST3, and AU plugins
            </p>
          </div>

          {/* Stats */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {isLoading ? (
              <>
                <Card>
                  <CardHeader className="pb-3">
                    <Skeleton className="h-4 w-24 mb-2" />
                    <Skeleton className="h-9 w-16" />
                  </CardHeader>
                  <CardContent>
                    <Skeleton className="h-3 w-32" />
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-3">
                    <Skeleton className="h-4 w-24 mb-2" />
                    <Skeleton className="h-9 w-16" />
                  </CardHeader>
                  <CardContent>
                    <Skeleton className="h-3 w-32" />
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-3">
                    <Skeleton className="h-4 w-24 mb-2" />
                    <Skeleton className="h-9 w-16" />
                  </CardHeader>
                  <CardContent>
                    <Skeleton className="h-3 w-32" />
                  </CardContent>
                </Card>
              </>
            ) : (
              <>
                <Card>
                  <CardHeader className="pb-3">
                    <CardDescription>Total Plugins</CardDescription>
                    <CardTitle className="text-3xl">{plugins.length}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-xs text-zinc-500">Available in library</div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-3">
                    <CardDescription>Loaded</CardDescription>
                    <CardTitle className="text-3xl">{loadedPlugins.length}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-xs text-zinc-500">Currently active</div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-3">
                    <CardDescription>Instruments</CardDescription>
                    <CardTitle className="text-3xl">
                      {plugins.filter((p) => p.category === "Instrument").length}
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-xs text-zinc-500">
                      {plugins.filter((p) => p.category !== "Instrument").length} effects
                    </div>
                  </CardContent>
                </Card>
              </>
            )}
          </div>

          {/* Loaded Plugins */}
          {loadedPlugins.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Power className="w-5 h-5 text-green-500" />
                  Loaded Plugins
                </CardTitle>
                <CardDescription>Currently active in your session</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {loadedPlugins.map((plugin) => (
                    <div
                      key={plugin.id}
                      className="flex items-center justify-between p-3 rounded-lg bg-zinc-900 border border-zinc-800"
                    >
                      <div className="flex items-center gap-3">
                        <Puzzle className="w-5 h-5 text-cyan-500" />
                        <div>
                          <div className="font-medium text-sm">{plugin.name}</div>
                          <div className="text-xs text-zinc-500">
                            {plugin.vendor} · v{plugin.version} · {plugin.format}
                          </div>
                        </div>
                      </div>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => togglePluginLoad(plugin.id)}
                      >
                        Unload
                      </Button>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Plugin Browser */}
          <Card>
            <CardHeader>
              <CardTitle>Plugin Library</CardTitle>
              <CardDescription>Browse and load available plugins</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Search and Filters */}
              <div className="flex gap-4">
                <div className="flex-1 relative">
                  <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                  <input
                    type="text"
                    placeholder="Search plugins..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="w-full pl-10 pr-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-cyan-500"
                  />
                </div>

                <Button variant="outline" size="sm">
                  <Download className="w-4 h-4 mr-2" />
                  Scan for Plugins
                </Button>
              </div>

              {/* Category Filters */}
              <div className="flex gap-2 flex-wrap">
                {CATEGORIES.map((category) => {
                  const count =
                    category === "All"
                      ? plugins.length
                      : plugins.filter((p) => p.category === category).length;

                  return (
                    <button
                      key={category}
                      onClick={() => setSelectedCategory(category)}
                      className={cn(
                        "px-3 py-1.5 rounded-lg text-sm font-medium transition-colors",
                        selectedCategory === category
                          ? "bg-cyan-500 text-white"
                          : "bg-zinc-900 text-zinc-400 hover:bg-zinc-800"
                      )}
                    >
                      {category}
                      <span className="ml-1.5 text-xs opacity-70">({count})</span>
                    </button>
                  );
                })}
              </div>

              {/* Plugin List */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3 max-h-[600px] overflow-y-auto pr-2">
                {isLoading ? (
                  <>
                    {Array.from({ length: 6 }).map((_, idx) => (
                      <div
                        key={idx}
                        className="p-4 rounded-lg border bg-zinc-900 border-zinc-800"
                      >
                        <div className="flex items-start justify-between mb-2">
                          <div className="flex-1">
                            <Skeleton className="h-4 w-32 mb-2" />
                            <Skeleton className="h-3 w-24" />
                          </div>
                          <Skeleton className="h-8 w-16 ml-2" />
                        </div>
                        <Skeleton className="h-5 w-20 mb-2" />
                        <Skeleton className="h-3 w-full mb-1" />
                        <Skeleton className="h-3 w-3/4" />
                      </div>
                    ))}
                  </>
                ) : (
                  filteredPlugins.map((plugin) => (
                  <div
                    key={plugin.id}
                    className={cn(
                      "p-4 rounded-lg border transition-all",
                      plugin.loaded
                        ? "bg-cyan-900/10 border-cyan-700/50"
                        : "bg-zinc-900 border-zinc-800 hover:border-zinc-700"
                    )}
                  >
                    <div className="flex items-start justify-between mb-2">
                      <div className="flex-1 min-w-0">
                        <h4 className="font-semibold text-sm truncate">{plugin.name}</h4>
                        <p className="text-xs text-zinc-500 mt-0.5">
                          {plugin.vendor} · v{plugin.version}
                        </p>
                      </div>
                      <Button
                        size="sm"
                        variant={plugin.loaded ? "secondary" : "outline"}
                        className="ml-2"
                        onClick={() => togglePluginLoad(plugin.id)}
                      >
                        {plugin.loaded ? "Loaded" : "Load"}
                      </Button>
                    </div>

                    {plugin.description && (
                      <p className="text-xs text-zinc-400 mb-2 line-clamp-2">
                        {plugin.description}
                      </p>
                    )}

                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-zinc-800 text-zinc-400">
                        {plugin.format}
                      </span>
                      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
                        {plugin.category}
                      </span>
                      {plugin.features?.slice(0, 2).map((feature) => (
                        <span
                          key={feature}
                          className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-zinc-800 text-zinc-500"
                        >
                          {feature}
                        </span>
                      ))}
                    </div>
                  </div>
                  ))
                )}
              </div>

              {!isLoading && filteredPlugins.length === 0 && (
                <div className="text-center py-12 text-zinc-500">
                  <Puzzle className="w-12 h-12 mx-auto mb-3 opacity-50" />
                  <p>No plugins found</p>
                  <p className="text-sm mt-1">Try adjusting your filters or search query</p>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </Layout>
  );
}
