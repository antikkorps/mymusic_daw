/**
 * Plugin Parameter Control Component
 * Provides UI for loading and controlling CLAP plugin parameters
 */

import React, { useState } from 'react';
import { usePluginControls, PluginParameter } from '../hooks/usePluginControls';

/**
 * Single parameter slider component
 */
interface ParameterSliderProps {
  pluginId: string;
  parameter: PluginParameter;
  currentValue: number;
  onValueChange: (parameterId: string, value: number) => void;
}

function ParameterSlider({ pluginId, parameter, currentValue, onValueChange }: ParameterSliderProps) {
  const [localValue, setLocalValue] = useState(currentValue);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseFloat(e.target.value);
    setLocalValue(newValue);
    onValueChange(parameter.id, newValue);
  };

  const handleReset = () => {
    setLocalValue(parameter.default);
    onValueChange(parameter.id, parameter.default);
  };

  // Format value for display
  const formatValue = (value: number): string => {
    if (parameter.unit) {
      return `${value.toFixed(2)} ${parameter.unit}`;
    }
    return value.toFixed(2);
  };

  return (
    <div style={styles.parameterRow}>
      <div style={styles.parameterInfo}>
        <label style={styles.parameterLabel}>{parameter.name}</label>
        <span style={styles.parameterValue}>{formatValue(localValue)}</span>
      </div>
      <div style={styles.parameterControl}>
        <input
          type="range"
          min={parameter.min}
          max={parameter.max}
          step={(parameter.max - parameter.min) / 1000}
          value={localValue}
          onChange={handleChange}
          style={styles.parameterSlider}
        />
        <button onClick={handleReset} style={styles.resetButton} title="Reset to default">
          ↺
        </button>
      </div>
      <div style={styles.parameterRange}>
        <span>{parameter.min}</span>
        <span>{parameter.max}</span>
      </div>
    </div>
  );
}

/**
 * Plugin instance panel component
 */
interface PluginPanelProps {
  pluginId: string;
  parameters: PluginParameter[];
  parameterValues: Map<string, number>;
  onUnload: () => void;
  onParameterChange: (parameterId: string, value: number) => void;
}

function PluginPanel({
  pluginId,
  parameters,
  parameterValues,
  onUnload,
  onParameterChange,
}: PluginPanelProps) {
  const [isExpanded, setIsExpanded] = useState(true);

  return (
    <div style={styles.pluginPanel}>
      <div style={styles.pluginHeader}>
        <div style={styles.pluginTitle}>
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            style={styles.expandButton}
            aria-label={isExpanded ? 'Collapse' : 'Expand'}
          >
            {isExpanded ? '▼' : '▶'}
          </button>
          <h3 style={styles.pluginName}>
            🎛️ {pluginId}
          </h3>
          <span style={styles.parameterCount}>
            {parameters.length} parameter{parameters.length !== 1 ? 's' : ''}
          </span>
        </div>
        <button onClick={onUnload} style={styles.unloadButton}>
          ✕ Unload
        </button>
      </div>

      {isExpanded && (
        <div style={styles.parametersContainer}>
          {parameters.length === 0 ? (
            <p style={styles.noParameters}>No parameters available</p>
          ) : (
            parameters.map((param) => (
              <ParameterSlider
                key={param.id}
                pluginId={pluginId}
                parameter={param}
                currentValue={parameterValues.get(param.id) ?? param.default}
                onValueChange={onParameterChange}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}

/**
 * Main plugin parameter control component
 */
export function PluginParameterControl() {
  const {
    loadedPlugins,
    isLoading,
    error,
    loadPlugin,
    unloadPlugin,
    setParameterValue,
  } = usePluginControls();

  const [pluginPath, setPluginPath] = useState('');
  const [loadError, setLoadError] = useState<string | null>(null);

  const handleLoadPlugin = async () => {
    if (!pluginPath.trim()) {
      setLoadError('Please enter a plugin path');
      return;
    }

    setLoadError(null);
    const result = await loadPlugin(pluginPath.trim());

    if (!result) {
      setLoadError('Failed to load plugin. Check console for details.');
    } else {
      setPluginPath(''); // Clear input on success
    }
  };

  const handleUnloadPlugin = async (pluginId: string) => {
    await unloadPlugin(pluginId);
  };

  const handleParameterChange = async (pluginId: string, parameterId: string, value: number) => {
    await setParameterValue(pluginId, parameterId, value);
  };

  // Common plugin paths for quick access (examples)
  const examplePaths = [
    '/Library/Audio/Plug-Ins/CLAP/Surge XT.clap',
    '/Library/Audio/Plug-Ins/CLAP/Vital.clap',
    'C:\\Program Files\\Common Files\\CLAP\\Surge XT.clap',
    'C:\\Program Files\\Common Files\\CLAP\\Vital.clap',
  ];

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>🎛️ CLAP Plugin Controller</h2>

      {/* Error Display */}
      {(error || loadError) && (
        <div style={styles.errorBox}>
          ⚠️ {error || loadError}
        </div>
      )}

      {/* Plugin Loader */}
      <div style={styles.loaderSection}>
        <h3 style={styles.sectionTitle}>📂 Load Plugin</h3>
        <div style={styles.loaderControl}>
          <input
            type="text"
            value={pluginPath}
            onChange={(e) => setPluginPath(e.target.value)}
            placeholder="Enter path to .clap plugin file..."
            style={styles.pathInput}
            disabled={isLoading}
          />
          <button
            onClick={handleLoadPlugin}
            style={styles.loadButton}
            disabled={isLoading || !pluginPath.trim()}
          >
            {isLoading ? '⏳ Loading...' : '📥 Load'}
          </button>
        </div>

        {/* Example Paths */}
        <details style={styles.examplesDetails}>
          <summary style={styles.examplesSummary}>💡 Common plugin paths</summary>
          <div style={styles.examplesList}>
            {examplePaths.map((path, index) => (
              <button
                key={index}
                onClick={() => setPluginPath(path)}
                style={styles.exampleButton}
              >
                {path}
              </button>
            ))}
          </div>
        </details>
      </div>

      {/* Loaded Plugins */}
      <div style={styles.pluginsSection}>
        <h3 style={styles.sectionTitle}>
          🎹 Loaded Plugins ({loadedPlugins.size})
        </h3>

        {loadedPlugins.size === 0 ? (
          <div style={styles.emptyState}>
            <p style={styles.emptyStateText}>No plugins loaded yet.</p>
            <p style={styles.emptyStateHint}>
              Load a CLAP plugin above to start controlling its parameters.
            </p>
          </div>
        ) : (
          <div style={styles.pluginsList}>
            {Array.from(loadedPlugins.entries()).map(([pluginId, plugin]) => (
              <PluginPanel
                key={pluginId}
                pluginId={pluginId}
                parameters={plugin.parameters}
                parameterValues={plugin.parameterValues}
                onUnload={() => handleUnloadPlugin(pluginId)}
                onParameterChange={(parameterId, value) =>
                  handleParameterChange(pluginId, parameterId, value)
                }
              />
            ))}
          </div>
        )}
      </div>

      {/* Info */}
      <div style={styles.infoBox}>
        <h4>ℹ️ About Plugin Control</h4>
        <ul style={styles.infoList}>
          <li>
            <strong>Load Plugin:</strong> Enter the full path to a .clap plugin file
          </li>
          <li>
            <strong>Parameters:</strong> Adjust plugin parameters in real-time using sliders
          </li>
          <li>
            <strong>Reset:</strong> Click the ↺ button to reset a parameter to its default value
          </li>
          <li>
            <strong>Unload:</strong> Remove a plugin from memory when you're done
          </li>
          <li>
            <strong>Multiple Plugins:</strong> You can load multiple plugin instances simultaneously
          </li>
        </ul>
      </div>
    </div>
  );
}

// Inline styles (matching DawEngineTest theme)
const styles = {
  container: {
    fontFamily: 'system-ui, -apple-system, sans-serif',
    maxWidth: '1000px',
    margin: '0 auto',
    padding: '20px',
    backgroundColor: '#1a1a1a',
    color: '#ffffff',
    borderRadius: '8px',
  },
  title: {
    fontSize: '28px',
    fontWeight: 'bold' as const,
    marginBottom: '20px',
    textAlign: 'center' as const,
  },
  errorBox: {
    padding: '12px',
    backgroundColor: '#dc2626',
    borderRadius: '4px',
    marginBottom: '20px',
    fontSize: '14px',
  },
  loaderSection: {
    marginBottom: '30px',
    padding: '20px',
    backgroundColor: '#2a2a2a',
    borderRadius: '6px',
  },
  sectionTitle: {
    fontSize: '20px',
    marginBottom: '15px',
    fontWeight: '600' as const,
  },
  loaderControl: {
    display: 'flex',
    gap: '10px',
    marginBottom: '15px',
  },
  pathInput: {
    flex: 1,
    padding: '10px 15px',
    backgroundColor: '#1a1a1a',
    color: '#ffffff',
    border: '2px solid #4a4a4a',
    borderRadius: '4px',
    fontSize: '14px',
    outline: 'none',
  },
  loadButton: {
    padding: '10px 20px',
    backgroundColor: '#3b82f6',
    color: '#ffffff',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 'bold' as const,
    transition: 'background-color 0.2s',
    whiteSpace: 'nowrap' as const,
  },
  examplesDetails: {
    marginTop: '10px',
  },
  examplesSummary: {
    cursor: 'pointer',
    fontSize: '14px',
    color: '#9ca3af',
    padding: '5px 0',
  },
  examplesList: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '5px',
    marginTop: '10px',
  },
  exampleButton: {
    padding: '8px 12px',
    backgroundColor: '#1a1a1a',
    color: '#9ca3af',
    border: '1px solid #4a4a4a',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '12px',
    textAlign: 'left' as const,
    fontFamily: 'monospace',
    transition: 'background-color 0.2s',
  },
  pluginsSection: {
    marginBottom: '30px',
  },
  emptyState: {
    padding: '40px',
    textAlign: 'center' as const,
    backgroundColor: '#2a2a2a',
    borderRadius: '6px',
    border: '2px dashed #4a4a4a',
  },
  emptyStateText: {
    fontSize: '18px',
    fontWeight: 'bold' as const,
    marginBottom: '10px',
  },
  emptyStateHint: {
    fontSize: '14px',
    color: '#9ca3af',
  },
  pluginsList: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '15px',
  },
  pluginPanel: {
    backgroundColor: '#2a2a2a',
    borderRadius: '6px',
    padding: '15px',
    border: '1px solid #4a4a4a',
  },
  pluginHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '15px',
  },
  pluginTitle: {
    display: 'flex',
    alignItems: 'center',
    gap: '10px',
  },
  expandButton: {
    backgroundColor: 'transparent',
    color: '#ffffff',
    border: 'none',
    cursor: 'pointer',
    fontSize: '14px',
    padding: '5px',
  },
  pluginName: {
    fontSize: '18px',
    fontWeight: '600' as const,
    margin: 0,
  },
  parameterCount: {
    fontSize: '12px',
    color: '#9ca3af',
    backgroundColor: '#1a1a1a',
    padding: '4px 8px',
    borderRadius: '12px',
  },
  unloadButton: {
    padding: '6px 12px',
    backgroundColor: '#dc2626',
    color: '#ffffff',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '12px',
    fontWeight: 'bold' as const,
    transition: 'background-color 0.2s',
  },
  parametersContainer: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '15px',
  },
  noParameters: {
    textAlign: 'center' as const,
    color: '#9ca3af',
    padding: '20px',
  },
  parameterRow: {
    backgroundColor: '#1a1a1a',
    padding: '12px',
    borderRadius: '4px',
  },
  parameterInfo: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '8px',
  },
  parameterLabel: {
    fontSize: '14px',
    fontWeight: '500' as const,
  },
  parameterValue: {
    fontSize: '14px',
    color: '#3b82f6',
    fontWeight: 'bold' as const,
  },
  parameterControl: {
    display: 'flex',
    alignItems: 'center',
    gap: '10px',
    marginBottom: '5px',
  },
  parameterSlider: {
    flex: 1,
    height: '6px',
    borderRadius: '3px',
    outline: 'none',
  },
  resetButton: {
    padding: '4px 8px',
    backgroundColor: '#4a4a4a',
    color: '#ffffff',
    border: 'none',
    borderRadius: '3px',
    cursor: 'pointer',
    fontSize: '14px',
    transition: 'background-color 0.2s',
  },
  parameterRange: {
    display: 'flex',
    justifyContent: 'space-between',
    fontSize: '11px',
    color: '#6b7280',
  },
  infoBox: {
    marginTop: '30px',
    padding: '20px',
    backgroundColor: '#2a2a2a',
    borderRadius: '6px',
    borderLeft: '4px solid #3b82f6',
  },
  infoList: {
    lineHeight: '1.8' as const,
    fontSize: '14px',
    color: '#d1d5db',
  },
};

export default PluginParameterControl;
