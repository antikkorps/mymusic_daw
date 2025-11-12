/**
 * React hook for CLAP Plugin Control
 * Provides functions to load, control, and manage CLAP plugin instances
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Plugin parameter interface
export interface PluginParameter {
  id: string;
  name: string;
  min: number;
  max: number;
  default: number;
  unit?: string;
}

// Loaded plugin info
export interface LoadedPluginInfo {
  id: string;
  name: string;
  vendor: string;
  version: string;
  parameterCount: number;
}

// Plugin instance with parameters
export interface PluginInstance {
  id: string;
  parameters: PluginParameter[];
  parameterValues: Map<string, number>;
}

// Hook return type
interface UsePluginControls {
  // State
  loadedPlugins: Map<string, PluginInstance>;
  isLoading: boolean;
  error: string | null;

  // Actions
  loadPlugin: (pluginPath: string) => Promise<string | null>;
  unloadPlugin: (pluginId: string) => Promise<void>;
  getParameters: (pluginId: string) => Promise<PluginParameter[]>;
  getParameterValue: (pluginId: string, parameterId: string) => Promise<number | null>;
  setParameterValue: (pluginId: string, parameterId: string, value: number) => Promise<void>;
  refreshPlugins: () => Promise<void>;
}

/**
 * Custom hook to interact with CLAP plugin instances
 *
 * @example
 * ```tsx
 * function PluginLoader() {
 *   const { loadPlugin, loadedPlugins, error } = usePluginControls();
 *
 *   const handleLoad = async () => {
 *     const pluginId = await loadPlugin('/path/to/plugin.clap');
 *     if (pluginId) {
 *       console.log('Plugin loaded:', pluginId);
 *     }
 *   };
 *
 *   return (
 *     <div>
 *       <button onClick={handleLoad}>Load Plugin</button>
 *       {error && <p>Error: {error}</p>}
 *       <p>Loaded plugins: {loadedPlugins.size}</p>
 *     </div>
 *   );
 * }
 * ```
 */
export function usePluginControls(): UsePluginControls {
  const [loadedPlugins, setLoadedPlugins] = useState<Map<string, PluginInstance>>(new Map());
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Load a CLAP plugin from a file path
   * @param pluginPath - Path to the .clap plugin file
   * @returns Plugin ID if successful, null otherwise
   */
  const loadPlugin = useCallback(async (pluginPath: string): Promise<string | null> => {
    setIsLoading(true);
    setError(null);

    try {
      // Load the plugin instance
      const pluginId = await invoke<string>('load_plugin_instance', { pluginPath });

      // Get plugin parameters
      const parameters = await invoke<PluginParameter[]>('get_plugin_parameters', { pluginId });

      // Fetch initial parameter values
      const parameterValues = new Map<string, number>();
      for (const param of parameters) {
        try {
          const value = await invoke<number>('get_plugin_parameter_value', {
            pluginId,
            parameterId: param.id,
          });
          parameterValues.set(param.id, value);
        } catch (err) {
          console.warn(`Failed to get value for parameter ${param.id}:`, err);
          parameterValues.set(param.id, param.default);
        }
      }

      // Add to loaded plugins
      setLoadedPlugins((prev) => {
        const updated = new Map(prev);
        updated.set(pluginId, {
          id: pluginId,
          parameters,
          parameterValues,
        });
        return updated;
      });

      console.log(`✅ Plugin loaded: ${pluginId} (${parameters.length} parameters)`);
      return pluginId;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to load plugin: ${errorMessage}`);
      console.error('Failed to load plugin:', err);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Unload a plugin instance
   * @param pluginId - ID of the plugin to unload
   */
  const unloadPlugin = useCallback(async (pluginId: string): Promise<void> => {
    setIsLoading(true);
    setError(null);

    try {
      await invoke('unload_plugin_instance', { pluginId });

      // Remove from loaded plugins
      setLoadedPlugins((prev) => {
        const updated = new Map(prev);
        updated.delete(pluginId);
        return updated;
      });

      console.log(`✅ Plugin unloaded: ${pluginId}`);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to unload plugin: ${errorMessage}`);
      console.error('Failed to unload plugin:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Get parameters for a loaded plugin
   * @param pluginId - ID of the plugin
   * @returns Array of plugin parameters
   */
  const getParameters = useCallback(async (pluginId: string): Promise<PluginParameter[]> => {
    setError(null);

    try {
      const parameters = await invoke<PluginParameter[]>('get_plugin_parameters', { pluginId });
      return parameters;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to get parameters: ${errorMessage}`);
      console.error('Failed to get parameters:', err);
      return [];
    }
  }, []);

  /**
   * Get the current value of a plugin parameter
   * @param pluginId - ID of the plugin
   * @param parameterId - ID of the parameter
   * @returns Current parameter value, or null if failed
   */
  const getParameterValue = useCallback(
    async (pluginId: string, parameterId: string): Promise<number | null> => {
      setError(null);

      try {
        const value = await invoke<number>('get_plugin_parameter_value', {
          pluginId,
          parameterId,
        });

        // Update local cache
        setLoadedPlugins((prev) => {
          const updated = new Map(prev);
          const plugin = updated.get(pluginId);
          if (plugin) {
            plugin.parameterValues.set(parameterId, value);
          }
          return updated;
        });

        return value;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(`Failed to get parameter value: ${errorMessage}`);
        console.error('Failed to get parameter value:', err);
        return null;
      }
    },
    []
  );

  /**
   * Set the value of a plugin parameter
   * @param pluginId - ID of the plugin
   * @param parameterId - ID of the parameter
   * @param value - New parameter value
   */
  const setParameterValue = useCallback(
    async (pluginId: string, parameterId: string, value: number): Promise<void> => {
      setError(null);

      try {
        // Get parameter info to validate range
        const plugin = loadedPlugins.get(pluginId);
        if (plugin) {
          const param = plugin.parameters.find((p) => p.id === parameterId);
          if (param) {
            // Clamp value to valid range
            const clampedValue = Math.max(param.min, Math.min(param.max, value));

            await invoke('set_plugin_parameter_value', {
              pluginId,
              parameterId,
              value: clampedValue,
            });

            // Update local cache
            setLoadedPlugins((prev) => {
              const updated = new Map(prev);
              const pluginInstance = updated.get(pluginId);
              if (pluginInstance) {
                pluginInstance.parameterValues.set(parameterId, clampedValue);
              }
              return updated;
            });

            console.log(`✅ Set ${parameterId} = ${clampedValue}`);
          } else {
            throw new Error(`Parameter not found: ${parameterId}`);
          }
        } else {
          throw new Error(`Plugin not found: ${pluginId}`);
        }
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(`Failed to set parameter value: ${errorMessage}`);
        console.error('Failed to set parameter value:', err);
      }
    },
    [loadedPlugins]
  );

  /**
   * Refresh the list of loaded plugins from the backend
   */
  const refreshPlugins = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);

    try {
      const pluginInfos = await invoke<LoadedPluginInfo[]>('get_loaded_plugins');

      // Rebuild the loaded plugins map
      const updatedPlugins = new Map<string, PluginInstance>();

      for (const info of pluginInfos) {
        // Get parameters for each plugin
        const parameters = await invoke<PluginParameter[]>('get_plugin_parameters', {
          pluginId: info.id,
        });

        // Get current values
        const parameterValues = new Map<string, number>();
        for (const param of parameters) {
          try {
            const value = await invoke<number>('get_plugin_parameter_value', {
              pluginId: info.id,
              parameterId: param.id,
            });
            parameterValues.set(param.id, value);
          } catch (err) {
            console.warn(`Failed to get value for parameter ${param.id}:`, err);
            parameterValues.set(param.id, param.default);
          }
        }

        updatedPlugins.set(info.id, {
          id: info.id,
          parameters,
          parameterValues,
        });
      }

      setLoadedPlugins(updatedPlugins);
      console.log(`✅ Refreshed ${updatedPlugins.size} loaded plugins`);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to refresh plugins: ${errorMessage}`);
      console.error('Failed to refresh plugins:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Initialize the hook - fetch loaded plugins on mount
   */
  useEffect(() => {
    refreshPlugins();
  }, []);

  return {
    loadedPlugins,
    isLoading,
    error,
    loadPlugin,
    unloadPlugin,
    getParameters,
    getParameterValue,
    setParameterValue,
    refreshPlugins,
  };
}

/**
 * Helper hook for managing a single plugin instance
 * Useful when you want to focus on one plugin at a time
 *
 * @example
 * ```tsx
 * function SinglePluginControl() {
 *   const plugin = useSinglePlugin('/path/to/plugin.clap');
 *
 *   if (!plugin.isLoaded) {
 *     return <p>Loading plugin...</p>;
 *   }
 *
 *   return (
 *     <div>
 *       <h2>Plugin: {plugin.id}</h2>
 *       {plugin.parameters.map(param => (
 *         <div key={param.id}>
 *           <label>{param.name}</label>
 *           <input
 *             type="range"
 *             min={param.min}
 *             max={param.max}
 *             value={plugin.getParamValue(param.id) ?? param.default}
 *             onChange={(e) => plugin.setParamValue(param.id, parseFloat(e.target.value))}
 *           />
 *         </div>
 *       ))}
 *     </div>
 *   );
 * }
 * ```
 */
export function useSinglePlugin(pluginPath: string | null) {
  const { loadPlugin, unloadPlugin, setParameterValue, loadedPlugins, error } = usePluginControls();
  const [pluginId, setPluginId] = useState<string | null>(null);
  const [isLoaded, setIsLoaded] = useState<boolean>(false);
  
  // Use a ref to track the current plugin ID for cleanup
  // This avoids stale closure issues in the cleanup function
  const pluginIdRef = useRef<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    if (pluginPath) {
      loadPlugin(pluginPath).then((id) => {
        if (isMounted && id) {
          setPluginId(id);
          setIsLoaded(true);
          pluginIdRef.current = id;
        }
      });

      // Cleanup on unmount or when pluginPath changes
      return () => {
        isMounted = false;
        const currentPluginId = pluginIdRef.current;
        if (currentPluginId) {
          unloadPlugin(currentPluginId);
          pluginIdRef.current = null;
        }
      };
    } else {
      // Reset state when pluginPath is null
      setPluginId(null);
      setIsLoaded(false);
    }
  }, [pluginPath, loadPlugin, unloadPlugin]);

  const plugin = pluginId ? loadedPlugins.get(pluginId) : undefined;

  const getParamValue = useCallback(
    (parameterId: string): number | undefined => {
      return plugin?.parameterValues.get(parameterId);
    },
    [plugin]
  );

  const setParamValue = useCallback(
    async (parameterId: string, value: number): Promise<void> => {
      if (pluginId) {
        await setParameterValue(pluginId, parameterId, value);
      }
    },
    [pluginId, setParameterValue]
  );

  return {
    id: pluginId,
    isLoaded,
    parameters: plugin?.parameters ?? [],
    getParamValue,
    setParamValue,
    error,
  };
}
