import { useState, useEffect, useRef } from "react";
import { Layout } from "~/components/layout/Layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Skeleton } from "~/components/ui/skeleton";
import { mockPlugins } from "~/lib/mockData";
import type { Plugin, PluginCategory, PluginFormat } from "~/types/plugin";
import { cn } from "~/lib/utils";
import { useToast } from "~/lib/toast";
import { invoke } from "@tauri-apps/api/core";
import { Puzzle, Download, Power, Search, Eye, EyeOff, Link2, Maximize2 } from "lucide-react";

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

interface PluginGuiInfo {
  is_visible: boolean;
  width: number;
  height: number;
  can_resize: boolean;
  api: string;
}

export default function PluginsPage() {
  console.log("🎯 PluginsPage component rendering");
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<PluginCategory | "All">("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isScanning, setIsScanning] = useState(false);
  const [guiInfo, setGuiInfo] = useState<Record<string, PluginGuiInfo>>({});
  const hasLoadedRef = useRef(false); // Prevent double loading in StrictMode
  const { showToast } = useToast();

  // Load plugins on mount (prevent double calls in StrictMode)
  useEffect(() => {
    console.log("🎯 useEffect triggered, hasLoadedRef.current:", hasLoadedRef.current);
    if (!hasLoadedRef.current) {
      console.log("🚀 Setting hasLoadedRef.current = true");
      hasLoadedRef.current = true;
      loadAvailablePlugins();
    } else {
      console.log("⏭️ Skipping - already loaded");
    }
  }, []);

  const loadAvailablePlugins = async () => {
    console.log("🚀 loadAvailablePlugins called");
    setIsLoading(true);
    try {
      // Try to scan for real plugins first
      console.log("📡 Calling scan_for_plugins...");
      const scannedPlugins = await invoke<Plugin[]>("scan_for_plugins");
      
      console.log("✅ Backend response received:", scannedPlugins);
      console.log("🔍 Scanned plugins:", scannedPlugins);
      console.log("📊 Plugin count:", scannedPlugins.length);
      console.log("📊 Type of response:", typeof scannedPlugins);
      console.log("📊 Is array:", Array.isArray(scannedPlugins));
      
      if (scannedPlugins && scannedPlugins.length > 0) {
        console.log("🔄 Converting scanned plugins to Plugin format...");
        // Convert scanned plugins to Plugin format
        const realPlugins: Plugin[] = scannedPlugins.map((info, index) => {
          console.log(`🔄 Processing plugin ${index}:`, info);
          const converted = {
            id: info.id,
            name: info.name,
            vendor: info.vendor,
            version: "1.0.0",
            format: "CLAP" as PluginFormat,
            category: "Effect" as PluginCategory,
            path: info.path, // Use actual path from backend
            description: "A CLAP plugin",
            features: ["GUI"],
            loaded: false,
            instanceId: undefined
          };
          console.log(`✅ Plugin ${index} converted:`, converted);
          return converted;
        });
        
        console.log("✅ All real plugins created:", realPlugins);
        console.log("🔄 Setting plugins with setPlugins...");
        setPlugins(realPlugins);
        console.log("✅ Plugins set successfully");
        showToast(`Found ${realPlugins.length} real plugins`, "success", 2000);
      } else {
        // Fallback to mock plugins
        console.log("⚠️ No real plugins found, using mock plugins");
        console.log("⚠️ scannedPlugins value:", scannedPlugins);
        console.log("⚠️ scannedPlugins type:", typeof scannedPlugins);
        console.log("⚠️ scannedPlugins is null:", scannedPlugins === null);
        console.log("⚠️ scannedPlugins is undefined:", scannedPlugins === undefined);
        setPlugins(mockPlugins);
        showToast("Using demo plugins (no real CLAP plugins found)", "info", 3000);
      }
      } catch (error) {
        console.error("❌ FAILED to scan plugins:", error);
        console.error("❌ Error type:", typeof error);
        console.error("❌ Error details:", JSON.stringify(error, null, 2));
        console.error("❌ Error stack:", error instanceof Error ? error.stack : "No stack");
        // Fallback to mock plugins
        console.log("🔄 Falling back to mock plugins");
        setPlugins(mockPlugins);
        showToast("Using demo plugins (scanning failed)", "warning", 3000);
      } finally {
        console.log("🏁 loadAvailablePlugins finished");
        setIsLoading(false);
      }
  };

  const handleScanPlugins = async () => {
    setIsScanning(true);
    try {
      const scannedPlugins = await invoke<Plugin[]>("scan_for_plugins");
      
      if (scannedPlugins.length > 0) {
        // Convert scanned plugins to Plugin format
        const realPlugins: Plugin[] = scannedPlugins.map(info => ({
          id: info.id,
          name: info.name,
          vendor: info.vendor,
          version: "1.0.0",
          format: "CLAP" as PluginFormat,
          category: "Effect" as PluginCategory,
          path: info.path, // Use actual path from backend
          description: "A CLAP plugin",
          features: ["GUI"],
          loaded: false,
          instanceId: undefined
        }));
        
        setPlugins(realPlugins);
        showToast(`Scanning complete: ${realPlugins.length} plugins found`, "success", 3000);
      } else {
        showToast("No CLAP plugins found in standard locations", "warning", 3000);
      }
    } catch (error) {
      console.error("Plugin scanning failed:", error);
      showToast(`Plugin scanning failed: ${error}`, "error", 3000);
    } finally {
      setIsScanning(false);
    }
  };

  const togglePluginLoad = async (pluginId: string) => {
    const plugin = plugins.find((p) => p.id === pluginId);
    if (!plugin) return;

    const isLoading = !plugin.loaded;

    if (isLoading) {
      // Try to load the plugin using the real Tauri command
// Load the actual plugin using its path from the scanned data
        const pluginPath = plugin.path; // Use the actual path from backend scan
        
      try {
          
        const instanceId = await invoke<string>("load_plugin_instance", {
          pluginPath,
          pluginId: plugin.id // Use the scanned plugin ID as the state key
        });
        
        setPlugins((prev) =>
          prev.map((p) =>
            p.id === pluginId
              ? { ...p, loaded: true, instanceId }
              : p
          )
        );

        showToast(`Plugin "${plugin.name}" loaded successfully`, "success");
      } catch (error) {
        console.error("Failed to load real plugin:", error);
        console.error("Error details:", JSON.stringify(error, null, 2));
        console.error("Plugin path:", pluginPath);
        console.error("Plugin ID:", plugin.id);
        
        // Fallback to mock behavior for demo
        setPlugins((prev) =>
          prev.map((p) =>
            p.id === pluginId
              ? { ...p, loaded: true, instanceId: `inst-${Date.now()}` }
              : p
          )
        );

        showToast(`Plugin "${plugin.name}" loaded (demo mode) - Error: ${error}`, "warning");
      }
    } else {
      // Unload plugin
      if (plugin.instanceId) {
        try {
          await invoke("unload_plugin_instance", {
            pluginId: plugin.instanceId
          });
        } catch (error) {
          console.error("Failed to unload plugin:", error);
        }
      }

      setPlugins((prev) =>
        prev.map((p) =>
          p.id === pluginId
            ? { ...p, loaded: false, instanceId: undefined }
            : p
        )
      );

      showToast(`Plugin "${plugin.name}" unloaded`, "info");
    }
  };

  const showPluginGui = async (instanceId: string) => {
    try {
      await invoke("show_plugin_gui", { pluginId: instanceId });
      showToast(`Plugin GUI shown`, "success");
      await updatePluginGuiInfo(instanceId);
    } catch (error) {
      showToast(`Failed to show GUI: ${error}`, "error");
    }
  };

  const hidePluginGui = async (instanceId: string) => {
    try {
      await invoke("hide_plugin_gui", { pluginId: instanceId });
      showToast(`Plugin GUI hidden`, "success");
      await updatePluginGuiInfo(instanceId);
    } catch (error) {
      showToast(`Failed to hide GUI: ${error}`, "error");
    }
  };

  const attachPluginGui = async (instanceId: string) => {
    try {
      const windowHandle = await invoke<string>("get_window_handle_for_plugin", {
        windowLabel: "main"
      });
      
      await invoke("attach_plugin_gui", {
        pluginId: instanceId,
        windowHandle
      });
      
      showToast(`Plugin GUI attached to window`, "success");
      await updatePluginGuiInfo(instanceId);
    } catch (error) {
      showToast(`Failed to attach GUI: ${error}`, "error");
    }
  };

  const updatePluginGuiInfo = async (instanceId: string) => {
    try {
      const [size, isVisible] = await Promise.all([
        invoke<[number, number]>("get_plugin_gui_size", { pluginId: instanceId }),
        invoke<boolean>("is_plugin_gui_visible", { pluginId: instanceId })
      ]);
      
      setGuiInfo(prev => ({
        ...prev,
        [instanceId]: {
          is_visible: isVisible,
          width: size[0],
          height: size[1],
          can_resize: true,
          api: "native"
        }
      }));
    } catch (error) {
      console.error(`Failed to update GUI info:`, error);
    }
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
                <div className="space-y-4">
                  {loadedPlugins.map((plugin) => {
                    const instanceId = plugin.instanceId;
                    const gui = instanceId ? guiInfo[instanceId] : null;
                    
                    return (
                      <div
                        key={plugin.id}
                        className="p-4 rounded-lg bg-zinc-900 border border-zinc-800"
                      >
                        <div className="flex items-center justify-between mb-3">
                          <div className="flex items-center gap-3">
                            <Puzzle className="w-5 h-5 text-cyan-500" />
                            <div>
                              <div className="font-medium text-sm">{plugin.name}</div>
                              <div className="text-xs text-zinc-500">
                                {plugin.vendor} · v{plugin.version} · {plugin.format}
                              </div>
                              {instanceId && (
                                <div className="text-xs text-zinc-600 font-mono">
                                  ID: {instanceId}
                                </div>
                              )}
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

                        {/* GUI Controls */}
                        {instanceId && (
                          <div className="space-y-3">
                            {/* GUI Status */}
                            {gui && (
                              <div className="p-2 bg-zinc-800 rounded border border-zinc-700 text-xs">
                                <div className="grid grid-cols-4 gap-2">
                                  <div>
                                    <span className="text-zinc-500">Status:</span>
                                    <div className="text-white">
                                      {gui.is_visible ? (
                                        <span className="text-green-500">Visible</span>
                                      ) : (
                                        <span className="text-zinc-400">Hidden</span>
                                      )}
                                    </div>
                                  </div>
                                  <div>
                                    <span className="text-zinc-500">Size:</span>
                                    <div className="text-white">
                                      {gui.width}×{gui.height}
                                    </div>
                                  </div>
                                  <div>
                                    <span className="text-zinc-500">API:</span>
                                    <div className="text-white">{gui.api}</div>
                                  </div>
                                  <div>
                                    <span className="text-zinc-500">Resize:</span>
                                    <div className="text-white">
                                      {gui.can_resize ? "Yes" : "No"}
                                    </div>
                                  </div>
                                </div>
                              </div>
                            )}

                            {/* GUI Action Buttons */}
                            <div className="grid grid-cols-2 gap-2">
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => showPluginGui(instanceId)}
                                className="flex items-center gap-1 h-8"
                              >
                                <Eye className="w-3 h-3" />
                                Show
                              </Button>
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => hidePluginGui(instanceId)}
                                className="flex items-center gap-1 h-8"
                              >
                                <EyeOff className="w-3 h-3" />
                                Hide
                              </Button>
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => attachPluginGui(instanceId)}
                                className="flex items-center gap-1 h-8"
                              >
                                <Link2 className="w-3 h-3" />
                                Attach
                              </Button>
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => {
                                  invoke("set_plugin_gui_size", {
                                    pluginId: instanceId,
                                    width: 800,
                                    height: 600
                                  }).then(() => updatePluginGuiInfo(instanceId));
                                }}
                                className="flex items-center gap-1 h-8"
                              >
                                <Maximize2 className="w-3 h-3" />
                                800×600
                              </Button>
                            </div>
                          </div>
                        )}
                      </div>
                    );
                  })}
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

                <Button 
                  variant="outline" 
                  size="sm"
                  onClick={handleScanPlugins}
                  disabled={isScanning}
                >
                  <Download className="w-4 h-4 mr-2" />
                  {isScanning ? "Scanning..." : "Scan for Plugins"}
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
