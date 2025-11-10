import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  addEdge,
  useEdgesState,
  useNodesState,
  type Connection,
  type Edge,
  type Node,
} from "@xyflow/react"
import "@xyflow/react/dist/style.css"
import { Power } from "lucide-react"
import { useCallback, useState } from "react"
import { Knob } from "~/components/audio/Knob"
import { Layout } from "~/components/layout/Layout"
import { Button } from "~/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card"
import { useToast } from "~/lib/toast"

const initialNodes: Node[] = [
  {
    id: "input",
    type: "input",
    data: { label: "Audio Input" },
    position: { x: 50, y: 200 },
    style: { background: "#06b6d4", color: "white", border: "2px solid #0891b2" },
  },
  {
    id: "filter",
    data: { label: "SVF Filter", type: "filter" },
    position: { x: 250, y: 100 },
    style: { background: "#3b82f6", color: "white", border: "2px solid #2563eb" },
  },
  {
    id: "delay",
    data: { label: "Stereo Delay", type: "delay" },
    position: { x: 250, y: 250 },
    style: { background: "#8b5cf6", color: "white", border: "2px solid #7c3aed" },
  },
  {
    id: "reverb",
    data: { label: "Plate Reverb", type: "reverb" },
    position: { x: 500, y: 175 },
    style: { background: "#ec4899", color: "white", border: "2px solid #db2777" },
  },
  {
    id: "output",
    type: "output",
    data: { label: "Audio Output" },
    position: { x: 750, y: 200 },
    style: { background: "#10b981", color: "white", border: "2px solid #059669" },
  },
]

const initialEdges: Edge[] = [
  { id: "e1", source: "input", target: "filter", animated: true },
  { id: "e2", source: "input", target: "delay", animated: true },
  { id: "e3", source: "filter", target: "reverb", animated: true },
  { id: "e4", source: "delay", target: "reverb", animated: true },
  { id: "e5", source: "reverb", target: "output", animated: true },
]

// Effect presets - defined outside component to prevent recreation
const filterPresets = {
  "Low Pass Warm": { cutoff: 800, resonance: 0.3, type: 0 },
  "High Pass Clean": { cutoff: 250, resonance: 0.1, type: 1 },
  "Band Pass Vocal": { cutoff: 2000, resonance: 0.6, type: 2 },
  "Resonant Sweep": { cutoff: 5000, resonance: 0.9, type: 0 },
}

const delayPresets = {
  Slapback: { time: 0.125, feedback: 0.2, mix: 0.2 },
  "Quarter Note": { time: 0.5, feedback: 0.4, mix: 0.3 },
  "Long Echo": { time: 1.0, feedback: 0.6, mix: 0.4 },
  "Ping Pong": { time: 0.375, feedback: 0.5, mix: 0.35 },
}

const reverbPresets = {
  "Small Room": { roomSize: 0.3, damping: 0.6, mix: 0.15 },
  "Medium Hall": { roomSize: 0.6, damping: 0.5, mix: 0.25 },
  "Large Cathedral": { roomSize: 0.9, damping: 0.3, mix: 0.35 },
  Plate: { roomSize: 0.7, damping: 0.7, mix: 0.2 },
}

export default function EffectsPage() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges)
  const [selectedNode, setSelectedNode] = useState<Node | null>(null)
  const { showToast } = useToast()

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  )

  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    setSelectedNode(node)
  }, [])

  // Effect parameters state
  const [filterParams, setFilterParams] = useState({
    cutoff: 1000,
    resonance: 0.7,
    type: 0,
  })
  const [delayParams, setDelayParams] = useState({ time: 0.375, feedback: 0.4, mix: 0.3 })
  const [reverbParams, setReverbParams] = useState({
    roomSize: 0.7,
    damping: 0.5,
    mix: 0.25,
  })

  // Bypass state for each effect
  const [bypassedEffects, setBypassedEffects] = useState<Record<string, boolean>>({
    filter: false,
    delay: false,
    reverb: false,
  })

  const toggleBypass = useCallback(
    (effectId: string, effectName: string) => {
      setBypassedEffects((prev) => {
        const newState = !prev[effectId]
        showToast(
          `${effectName} ${newState ? "bypassed" : "enabled"}`,
          newState ? "warning" : "success",
          1500
        )
        return { ...prev, [effectId]: newState }
      })
    },
    [showToast]
  )

  const loadPreset = useCallback(
    (effectType: string, presetName: string) => {
      if (effectType === "filter") {
        const preset = filterPresets[presetName as keyof typeof filterPresets]
        if (preset) {
          setFilterParams(preset)
          showToast(`Loaded preset: ${presetName}`, "success", 1500)
        }
      } else if (effectType === "delay") {
        const preset = delayPresets[presetName as keyof typeof delayPresets]
        if (preset) {
          setDelayParams(preset)
          showToast(`Loaded preset: ${presetName}`, "success", 1500)
        }
      } else if (effectType === "reverb") {
        const preset = reverbPresets[presetName as keyof typeof reverbPresets]
        if (preset) {
          setReverbParams(preset)
          showToast(`Loaded preset: ${presetName}`, "success", 1500)
        }
      }
    },
    [showToast]
  )

  const renderEffectControls = (): React.ReactNode => {
    if (
      !selectedNode ||
      selectedNode.type === "input" ||
      selectedNode.type === "output"
    ) {
      return (
        <div className="text-center text-zinc-500 py-8">
          Select an effect node to edit parameters
        </div>
      )
    }

    const effectType = selectedNode.data.type as string
    const isBypassed = bypassedEffects[selectedNode.id]

    switch (effectType) {
      case "filter":
        return (
          <div
            className={`space-y-6 transition-opacity ${isBypassed ? "opacity-40" : ""}`}
          >
            {isBypassed && (
              <div className="text-xs text-amber-500 text-center py-2 bg-amber-500/10 rounded border border-amber-500/20">
                Effect is bypassed
              </div>
            )}

            {/* Preset selector */}
            <div>
              <label className="text-xs text-zinc-500 mb-2 block">Presets</label>
              <select
                className="w-full text-xs bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-zinc-300 hover:border-zinc-600 focus:border-cyan-500 focus:outline-none"
                onChange={(e) => loadPreset("filter", e.target.value)}
                defaultValue=""
              >
                <option value="" disabled>
                  Load a preset...
                </option>
                {Object.keys(filterPresets).map((presetName) => (
                  <option key={presetName} value={presetName}>
                    {presetName}
                  </option>
                ))}
              </select>
            </div>

            <div className="flex justify-around">
              <Knob
                value={filterParams.cutoff}
                onChange={(v) => setFilterParams({ ...filterParams, cutoff: v })}
                min={20}
                max={20000}
                label="Cutoff"
                valueDisplay={(v) => `${v.toFixed(0)}Hz`}
              />
              <Knob
                value={filterParams.resonance}
                onChange={(v) => setFilterParams({ ...filterParams, resonance: v })}
                min={0}
                max={1}
                label="Resonance"
                valueDisplay={(v) => v.toFixed(2)}
              />
            </div>
            <div>
              <label className="text-xs text-zinc-500 mb-2 block">Filter Type</label>
              <div className="grid grid-cols-3 gap-2">
                {["Low Pass", "High Pass", "Band Pass"].map((type, idx) => (
                  <Button
                    key={type}
                    size="sm"
                    variant={filterParams.type === idx ? "secondary" : "outline"}
                    onClick={() => setFilterParams({ ...filterParams, type: idx })}
                  >
                    {type}
                  </Button>
                ))}
              </div>
            </div>
          </div>
        )

      case "delay":
        return (
          <div
            className={`space-y-6 transition-opacity ${isBypassed ? "opacity-40" : ""}`}
          >
            {isBypassed && (
              <div className="text-xs text-amber-500 text-center py-2 bg-amber-500/10 rounded border border-amber-500/20">
                Effect is bypassed
              </div>
            )}

            {/* Preset selector */}
            <div>
              <label className="text-xs text-zinc-500 mb-2 block">Presets</label>
              <select
                className="w-full text-xs bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-zinc-300 hover:border-zinc-600 focus:border-cyan-500 focus:outline-none"
                onChange={(e) => loadPreset("delay", e.target.value)}
                defaultValue=""
              >
                <option value="" disabled>
                  Load a preset...
                </option>
                {Object.keys(delayPresets).map((presetName) => (
                  <option key={presetName} value={presetName}>
                    {presetName}
                  </option>
                ))}
              </select>
            </div>

            <div className="flex justify-around">
              <Knob
                value={delayParams.time}
                onChange={(v) => setDelayParams({ ...delayParams, time: v })}
                min={0}
                max={2}
                label="Time"
                valueDisplay={(v) => `${(v * 1000).toFixed(0)}ms`}
              />
              <Knob
                value={delayParams.feedback}
                onChange={(v) => setDelayParams({ ...delayParams, feedback: v })}
                min={0}
                max={0.95}
                label="Feedback"
                valueDisplay={(v) => `${(v * 100).toFixed(0)}%`}
              />
              <Knob
                value={delayParams.mix}
                onChange={(v) => setDelayParams({ ...delayParams, mix: v })}
                min={0}
                max={1}
                label="Mix"
                valueDisplay={(v) => `${(v * 100).toFixed(0)}%`}
              />
            </div>
          </div>
        )

      case "reverb":
        return (
          <div
            className={`space-y-6 transition-opacity ${isBypassed ? "opacity-40" : ""}`}
          >
            {isBypassed && (
              <div className="text-xs text-amber-500 text-center py-2 bg-amber-500/10 rounded border border-amber-500/20">
                Effect is bypassed
              </div>
            )}

            {/* Preset selector */}
            <div>
              <label className="text-xs text-zinc-500 mb-2 block">Presets</label>
              <select
                className="w-full text-xs bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-zinc-300 hover:border-zinc-600 focus:border-cyan-500 focus:outline-none"
                onChange={(e) => loadPreset("reverb", e.target.value)}
                defaultValue=""
              >
                <option value="" disabled>
                  Load a preset...
                </option>
                {Object.keys(reverbPresets).map((presetName) => (
                  <option key={presetName} value={presetName}>
                    {presetName}
                  </option>
                ))}
              </select>
            </div>

            <div className="flex justify-around">
              <Knob
                value={reverbParams.roomSize}
                onChange={(v) => setReverbParams({ ...reverbParams, roomSize: v })}
                min={0}
                max={1}
                label="Room Size"
                valueDisplay={(v) => `${(v * 100).toFixed(0)}%`}
              />
              <Knob
                value={reverbParams.damping}
                onChange={(v) => setReverbParams({ ...reverbParams, damping: v })}
                min={0}
                max={1}
                label="Damping"
                valueDisplay={(v) => `${(v * 100).toFixed(0)}%`}
              />
              <Knob
                value={reverbParams.mix}
                onChange={(v) => setReverbParams({ ...reverbParams, mix: v })}
                min={0}
                max={1}
                label="Mix"
                valueDisplay={(v) => `${(v * 100).toFixed(0)}%`}
              />
            </div>
          </div>
        )

      default:
        return null
    }
  }

  return (
    <Layout>
      <div className="h-full flex gap-4 p-4 bg-zinc-900">
        {/* React Flow Canvas */}
        <div className="flex-1 rounded-lg border border-zinc-800 overflow-hidden bg-zinc-950">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={onNodeClick}
            fitView
          >
            <Background color="#27272a" gap={16} />
            <Controls />
            <MiniMap
              nodeColor={(node) => {
                const style = node.style as any
                return style?.background || "#3b82f6"
              }}
              maskColor="rgba(0, 0, 0, 0.5)"
            />
          </ReactFlow>
        </div>

        {/* Effect Controls Panel */}
        <div className="w-80 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Effect Chain</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="text-xs text-zinc-500">
                Click on effect nodes to edit parameters
              </div>
              <div className="space-y-1">
                {initialNodes
                  .filter((n) => n.type !== "input" && n.type !== "output")
                  .map((node) => {
                    const effectId = node.id
                    const isBypassed = bypassedEffects[effectId]
                    return (
                      <div
                        key={node.id}
                        className={`flex items-center justify-between p-2 rounded border transition-all ${
                          isBypassed
                            ? "bg-zinc-900/50 border-zinc-700 opacity-60"
                            : "bg-zinc-900 border-zinc-800"
                        }`}
                      >
                        <span
                          className={`text-sm ${isBypassed ? "line-through text-zinc-600" : ""}`}
                        >
                          {(node.data as { label: string }).label}
                        </span>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="h-6 w-6 p-0"
                          onClick={() =>
                            toggleBypass(effectId, (node.data as { label: string }).label)
                          }
                        >
                          <Power
                            className={`w-3 h-3 transition-colors ${
                              isBypassed ? "text-zinc-600" : "text-green-500"
                            }`}
                          />
                        </Button>
                      </div>
                    )
                  })}
              </div>
            </CardContent>
          </Card>

          {selectedNode && (
            <Card>
              <CardHeader>
                <CardTitle>{(selectedNode.data as { label: string }).label}</CardTitle>
              </CardHeader>
              <CardContent>{renderEffectControls()}</CardContent>
            </Card>
          )}
        </div>
      </div>
    </Layout>
  )
}
