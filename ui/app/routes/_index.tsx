import { Layout } from "~/components/layout/Layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Waveform } from "~/components/audio/Waveform";
import { mockTracks, mockPlugins } from "~/lib/mockData";
import { FileAudio, Music, Puzzle, Clock, Activity, Cpu } from "lucide-react";
import { Link } from "react-router";
import { useDawEngine } from "~/hooks/useDawEngine";

export default function DashboardPage() {
  const { engineStatus, isEngineReady, error } = useDawEngine();
  const loadedPlugins = mockPlugins.filter((p) => p.loaded);
  const totalTracks = mockTracks.length;
  const audioTracks = mockTracks.filter((t) => t.type === "audio").length;
  const midiTracks = mockTracks.filter((t) => t.type === "midi").length;

  return (
    <Layout>
      <div className="h-full bg-zinc-900 p-6 overflow-auto">
        <div className="max-w-7xl mx-auto space-y-6">
          {/* Header */}
          <div>
            <h2 className="text-3xl font-bold text-white">Dashboard</h2>
            <p className="text-zinc-400 mt-1">Welcome to your DAW workspace</p>
          </div>

          {/* Quick Stats */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <Card>
              <CardHeader className="pb-3">
                <CardDescription>Total Tracks</CardDescription>
                <CardTitle className="text-3xl">{totalTracks}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-xs text-zinc-500">
                  {audioTracks} audio, {midiTracks} MIDI
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="pb-3">
                <CardDescription>Active Plugins</CardDescription>
                <CardTitle className="text-3xl">{loadedPlugins.length}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-xs text-zinc-500">
                  {mockPlugins.length} total available
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="pb-3">
                <CardDescription>Sample Rate</CardDescription>
                <CardTitle className="text-3xl">48k</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-xs text-zinc-500">24-bit depth</div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="pb-3">
                <CardDescription>Latency</CardDescription>
                <CardTitle className="text-3xl">5.8ms</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-xs text-zinc-500">256 buffer size</div>
              </CardContent>
            </Card>
          </div>

          {/* Rust Engine Status - TEST BRIDGE */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Cpu className="w-5 h-5 text-green-500" />
                Rust Audio Engine
              </CardTitle>
              <CardDescription>Connection test: React ↔ Rust (Tauri)</CardDescription>
            </CardHeader>
            <CardContent>
              {error ? (
                <p className="text-red-400">❌ Error: {error}</p>
              ) : engineStatus ? (
                <div className="space-y-3">
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <p className="text-xs text-zinc-500">Name</p>
                      <p className="font-medium">{engineStatus.name}</p>
                    </div>
                    <div>
                      <p className="text-xs text-zinc-500">Version</p>
                      <p className="font-medium">{engineStatus.version}</p>
                    </div>
                    <div>
                      <p className="text-xs text-zinc-500">Status</p>
                      <p className="font-medium text-green-400">✓ {engineStatus.status}</p>
                    </div>
                    
                  </div>

                  <div className="pt-3 border-t border-zinc-800">
                    <p className="text-xs text-zinc-500 mb-2">Bridge Status</p>
                    <div className={`px-2 py-1 rounded text-xs font-medium ${
                      isEngineReady 
                        ? 'bg-green-900 text-green-300' 
                        : 'bg-yellow-900 text-yellow-300'
                    }`}>
                      {isEngineReady ? 'Ready' : 'Initializing...'}
                    </div>
                  </div>
                </div>
              ) : (
                <p className="text-zinc-400">No engine info available</p>
              )}
            </CardContent>
          </Card>

          {/* Master Output Waveform */}
          <Card>
            <CardHeader>
              <CardTitle>Master Output</CardTitle>
              <CardDescription>Real-time waveform visualization</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="h-48 bg-zinc-950 rounded-lg border border-zinc-800">
                <Waveform color="#06b6d4" animate />
              </div>
            </CardContent>
          </Card>

          {/* Synthesizer Demo Link */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Music className="w-5 h-5 text-purple-500" />
                Synthesizer Demo
              </CardTitle>
              <CardDescription>Interactive synthesizer with real-time controls</CardDescription>
            </CardHeader>
            <CardContent>
              <Link to="/synthesizer-demo">
                <Button className="w-full h-16 text-lg bg-purple-600 hover:bg-purple-700">
                  🎹 Launch Synthesizer Demo
                </Button>
              </Link>
              <p className="text-xs text-zinc-500 mt-2">
                Full synthesizer with ADSR, LFO, filters, and real-time monitoring
              </p>
            </CardContent>
          </Card>

          {/* Quick Actions */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Activity className="w-5 h-5 text-cyan-500" />
                  Recent Activity
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  {mockTracks.slice(0, 5).map((track) => (
                    <div
                      key={track.id}
                      className="flex items-center gap-3 p-2 rounded bg-zinc-900"
                    >
                      <div
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: track.color }}
                      />
                      <div className="flex-1">
                        <div className="text-sm font-medium">{track.name}</div>
                        <div className="text-xs text-zinc-500">
                          {track.clips.length} clips · {track.effects.length} effects
                        </div>
                      </div>
                      <div className="text-xs text-zinc-600">{track.type}</div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Quick Actions</CardTitle>
                <CardDescription>Navigate to different views</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-2 gap-3">
                  <Link to="/mixer">
                    <Button variant="outline" className="w-full h-20 flex-col gap-2">
                      <FileAudio className="w-6 h-6" />
                      <span className="text-sm">Mixer</span>
                    </Button>
                  </Link>

                  <Link to="/piano-roll">
                    <Button variant="outline" className="w-full h-20 flex-col gap-2">
                      <Music className="w-6 h-6" />
                      <span className="text-sm">Piano Roll</span>
                    </Button>
                  </Link>

                  <Link to="/timeline">
                    <Button variant="outline" className="w-full h-20 flex-col gap-2">
                      <Clock className="w-6 h-6" />
                      <span className="text-sm">Timeline</span>
                    </Button>
                  </Link>

                  <Link to="/plugins">
                    <Button variant="outline" className="w-full h-20 flex-col gap-2">
                      <Puzzle className="w-6 h-6" />
                      <span className="text-sm">Plugins</span>
                    </Button>
                  </Link>
                </div>
              </CardContent>
            </Card>
          </div>

          {/* Loaded Plugins */}
          {loadedPlugins.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle>Loaded Plugins</CardTitle>
                <CardDescription>Currently active in your session</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                  {loadedPlugins.map((plugin) => (
                    <div
                      key={plugin.id}
                      className="p-3 rounded-lg bg-zinc-900 border border-zinc-800"
                    >
                      <div className="font-medium text-sm">{plugin.name}</div>
                      <div className="text-xs text-zinc-500 mt-1">
                        {plugin.vendor} · v{plugin.version}
                      </div>
                      <div className="mt-2">
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
                          {plugin.category}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </Layout>
  );
}
