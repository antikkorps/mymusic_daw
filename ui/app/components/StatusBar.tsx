/**
 * Status Bar Component
 * Displays system status, errors, and performance metrics
 */

import React from 'react';
import { usePerformanceMonitor } from '../hooks/useAudioEvents';
import { useToastStore } from './ErrorHandling';

export function StatusBar() {
  const { 
    cpuUsage, 
    activeVoicesCount, 
    getCpuStatusColor, 
    getCpuStatusLevel,
    getSystemHealth,
    isConnected 
  } = usePerformanceMonitor();
  
  const { errors, hasErrors, clearErrors } = useToastStore();

  const getSystemHealthColor = (health: string) => {
    switch (health) {
      case 'excellent': return 'text-green-500';
      case 'good': return 'text-blue-500';
      case 'fair': return 'text-yellow-500';
      case 'poor': return 'text-red-500';
      default: return 'text-gray-500';
    }
  };

  const getSystemHealthIcon = (health: string) => {
    switch (health) {
      case 'excellent': return '🟢';
      case 'good': return '🔵';
      case 'fair': return '🟡';
      case 'poor': return '🔴';
      default: return '⚪';
    }
  };

  const systemHealth = getSystemHealth();
  const errorCount = errors.filter(e => e.type === 'error').length;
  const warningCount = errors.filter(e => e.type === 'warning').length;

  return (
    <div className="bg-gray-800 border-t border-gray-700 px-4 py-2 text-sm text-gray-300">
      <div className="flex items-center justify-between">
        {/* Left side - Connection and System Health */}
        <div className="flex items-center space-x-4">
          {/* Connection Status */}
          <div className="flex items-center space-x-2">
            <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-xs">
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>

          {/* System Health */}
          <div className={`flex items-center space-x-1 ${getSystemHealthColor(systemHealth)}`}>
            <span>{getSystemHealthIcon(systemHealth)}</span>
            <span className="text-xs capitalize">{systemHealth}</span>
          </div>

          {/* CPU Usage */}
          <div className={`flex items-center space-x-1 ${getCpuStatusColor()}`}>
            <span className="text-xs">CPU:</span>
            <span className="text-xs font-mono">{cpuUsage.toFixed(1)}%</span>
          </div>

          {/* Active Voices */}
          <div className="flex items-center space-x-1">
            <span className="text-xs">Voices:</span>
            <span className="text-xs font-mono">{activeVoicesCount}/16</span>
          </div>
        </div>

        {/* Right side - Errors and Actions */}
        <div className="flex items-center space-x-4">
          {/* Error Summary */}
          {(errorCount > 0 || warningCount > 0) && (
            <div className="flex items-center space-x-2">
              {errorCount > 0 && (
                <div className="flex items-center space-x-1 text-red-400">
                  <span>❌</span>
                  <span className="text-xs">{errorCount}</span>
                </div>
              )}
              {warningCount > 0 && (
                <div className="flex items-center space-x-1 text-yellow-400">
                  <span>⚠️</span>
                  <span className="text-xs">{warningCount}</span>
                </div>
              )}
              {hasErrors() && (
                <button
                  onClick={clearErrors}
                  className="text-xs text-gray-400 hover:text-white transition-colors"
                >
                  Clear
                </button>
              )}
            </div>
          )}

          {/* Performance Indicator */}
          <div className="text-xs text-gray-500">
            {getCpuStatusLevel() === 'critical' && '⚠️ High CPU Load'}
            {getCpuStatusLevel() === 'warning' && '⚡ Moderate Load'}
            {getCpuStatusLevel() === 'good' && '✅ Optimal'}
          </div>
        </div>
      </div>

      {/* Detailed Error Preview (only when there are errors) */}
      {hasErrors() && (
        <div className="mt-2 pt-2 border-t border-gray-700 max-h-20 overflow-y-auto">
          <div className="space-y-1">
            {errors.slice(-3).map((error) => (
              <div key={error.id} className="flex items-center justify-between text-xs">
                <div className="flex items-center space-x-2">
                  <span>
                    {error.type === 'error' ? '❌' : error.type === 'warning' ? '⚠️' : 'ℹ️'}
                  </span>
                  <span className="truncate max-w-xs">{error.message}</span>
                </div>
                <span className="text-gray-500">
                  {new Date(error.timestamp).toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Compact Status Bar for smaller screens
 */
export function CompactStatusBar() {
  const { cpuUsage, isConnected, getCpuStatusLevel } = usePerformanceMonitor();
  const { hasErrors } = useToastStore();

  return (
    <div className="bg-gray-800 border-t border-gray-700 px-2 py-1 text-xs text-gray-400">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
          <span>CPU: {cpuUsage.toFixed(0)}%</span>
          {hasErrors() && <span className="text-red-400">⚠️</span>}
        </div>
        <div className="text-gray-500">
          {getCpuStatusLevel() === 'critical' && 'High Load'}
          {getCpuStatusLevel() === 'warning' && 'Moderate'}
          {getCpuStatusLevel() === 'good' && 'Good'}
        </div>
      </div>
    </div>
  );
}

/**
 * Performance Panel Component
 * Detailed performance metrics for advanced users
 */
export function PerformancePanel() {
  const { 
    cpuUsage, 
    activeVoicesCount, 
    audioLevel, 
    getCpuStatusColor, 
    getCpuStatusLevel,
    getSystemHealth,
    getPeakLevel,
    isClipping,
    getVoiceUsagePercentage,
    isConnected 
  } = usePerformanceMonitor();

  const { errors } = useToastStore();

  return (
    <div className="bg-gray-900 border border-gray-700 rounded-lg p-4 space-y-4">
      <h3 className="text-white font-semibold">Performance Monitor</h3>
      
      {/* System Status */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <h4 className="text-gray-400 text-sm mb-2">System Status</h4>
          <div className="space-y-1 text-sm">
            <div className="flex justify-between">
              <span>Connection:</span>
              <span className={isConnected ? 'text-green-400' : 'text-red-400'}>
                {isConnected ? 'Connected' : 'Disconnected'}
              </span>
            </div>
            <div className="flex justify-between">
              <span>Health:</span>
              <span className={getSystemHealthColor(getSystemHealth())}>
                {getSystemHealth()}
              </span>
            </div>
          </div>
        </div>

        <div>
          <h4 className="text-gray-400 text-sm mb-2">CPU Performance</h4>
          <div className="space-y-1 text-sm">
            <div className="flex justify-between">
              <span>Usage:</span>
              <span className={getCpuStatusColor()}>
                {cpuUsage.toFixed(1)}%
              </span>
            </div>
            <div className="flex justify-between">
              <span>Status:</span>
              <span className="capitalize">{getCpuStatusLevel()}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Audio Performance */}
      <div>
        <h4 className="text-gray-400 text-sm mb-2">Audio Performance</h4>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div className="space-y-1">
            <div className="flex justify-between">
              <span>Active Voices:</span>
              <span>{activeVoicesCount}/16</span>
            </div>
            <div className="flex justify-between">
              <span>Voice Usage:</span>
              <span>{getVoiceUsagePercentage().toFixed(0)}%</span>
            </div>
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <span>Peak Level:</span>
              <span className={isClipping() ? 'text-red-400' : 'text-green-400'}>
                {(getPeakLevel() * 100).toFixed(1)}%
              </span>
            </div>
            <div className="flex justify-between">
              <span>Clipping:</span>
              <span>{isClipping() ? 'Yes' : 'No'}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Error Summary */}
      {errors.length > 0 && (
        <div>
          <h4 className="text-gray-400 text-sm mb-2">Recent Errors</h4>
          <div className="space-y-1 max-h-20 overflow-y-auto">
            {errors.slice(-5).map((error) => (
              <div key={error.id} className="text-xs text-gray-300 flex items-center space-x-2">
                <span>
                  {error.type === 'error' ? '❌' : error.type === 'warning' ? '⚠️' : 'ℹ️'}
                </span>
                <span className="truncate">{error.message}</span>
                <span className="text-gray-500">
                  {new Date(error.timestamp).toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Visual Indicators */}
      <div className="space-y-2">
        <div>
          <div className="flex justify-between text-sm mb-1">
            <span>CPU Usage</span>
            <span>{cpuUsage.toFixed(1)}%</span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-2">
            <div 
              className={`h-2 rounded-full transition-all ${
                cpuUsage < 50 ? 'bg-green-500' : 
                cpuUsage < 75 ? 'bg-yellow-500' : 'bg-red-500'
              }`}
              style={{ width: `${Math.min(cpuUsage, 100)}%` }}
            />
          </div>
        </div>

        <div>
          <div className="flex justify-between text-sm mb-1">
            <span>Voice Usage</span>
            <span>{getVoiceUsagePercentage().toFixed(0)}%</span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-2">
            <div 
              className={`h-2 rounded-full transition-all ${
                getVoiceUsagePercentage() < 50 ? 'bg-green-500' : 
                getVoiceUsagePercentage() < 75 ? 'bg-yellow-500' : 'bg-red-500'
              }`}
              style={{ width: `${Math.min(getVoiceUsagePercentage(), 100)}%` }}
            />
          </div>
        </div>

        <div>
          <div className="flex justify-between text-sm mb-1">
            <span>Audio Level</span>
            <span>{(getPeakLevel() * 100).toFixed(1)}%</span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-2">
            <div 
              className={`h-2 rounded-full transition-all ${
                getPeakLevel() < 0.85 ? 'bg-green-500' : 
                getPeakLevel() < 0.95 ? 'bg-yellow-500' : 'bg-red-500'
              }`}
              style={{ width: `${Math.min(getPeakLevel() * 100, 100)}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}