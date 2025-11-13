/**
 * Global Error Boundary and Toast Notification System
 * Provides centralized error handling for the MyMusic DAW interface
 */

import React, { Component, ErrorInfo, ReactNode, useCallback } from 'react';
import { create } from 'zustand';

// Error types
export interface ErrorInfo {
  id: string;
  message: string;
  type: 'error' | 'warning' | 'info' | 'success';
  timestamp: number;
  source: 'audio' | 'midi' | 'ui' | 'system' | 'user';
  details?: any;
  action?: {
    label: string;
    handler: () => void;
  };
}

// Toast store for global error management
interface ToastStore {
  errors: ErrorInfo[];
  addError: (error: Omit<ErrorInfo, 'id' | 'timestamp'>) => void;
  removeError: (id: string) => void;
  clearErrors: () => void;
  hasErrors: () => boolean;
  getErrorsByType: (type: ErrorInfo['type']) => ErrorInfo[];
  getErrorsBySource: (source: ErrorInfo['source']) => ErrorInfo[];
}

export const useToastStore = create<ToastStore>((set, get) => ({
  errors: [],
  
  addError: (error) => {
    const newError: ErrorInfo = {
      ...error,
      id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      timestamp: Date.now(),
    };
    
    set((state) => ({
      errors: [...state.errors.slice(-9), newError], // Keep last 10 errors
    }));
    
    // Auto-remove success and info messages after 5 seconds
    if (error.type === 'success' || error.type === 'info') {
      setTimeout(() => {
        get().removeError(newError.id);
      }, 5000);
    }
  },
  
  removeError: (id) => {
    set((state) => ({
      errors: state.errors.filter((error) => error.id !== id),
    }));
  },
  
  clearErrors: () => {
    set({ errors: [] });
  },
  
  hasErrors: () => {
    return get().errors.some((error) => error.type === 'error' || error.type === 'warning');
  },
  
  getErrorsByType: (type) => {
    return get().errors.filter((error) => error.type === type);
  },
  
  getErrorsBySource: (source) => {
    return get().errors.filter((error) => error.source === source);
  },
}));

// Error Boundary Component
interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.setState({ errorInfo });
    
    // Add error to toast store
    const { addError } = useToastStore.getState();
    addError({
      message: `UI Error: ${error.message}`,
      type: 'error',
      source: 'ui',
      details: {
        error: error.toString(),
        stack: error.stack,
        componentStack: errorInfo.componentStack,
      },
      action: {
        label: 'Reload',
        handler: () => window.location.reload(),
      },
    });

    // Call custom error handler if provided
    if (this.props.onError) {
      this.props.onError(error, errorInfo);
    }

    // Log to console for debugging
    console.error('Error Boundary caught an error:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="min-h-screen bg-gray-900 text-white flex items-center justify-center p-4">
          <div className="max-w-md w-full bg-gray-800 rounded-lg p-6 border border-red-500">
            <h2 className="text-xl font-bold text-red-400 mb-4">
              Something went wrong
            </h2>
            <p className="text-gray-300 mb-4">
              The MyMusic DAW interface encountered an unexpected error. 
              You can try reloading the page or continue with limited functionality.
            </p>
            <div className="space-y-2">
              <button
                onClick={() => window.location.reload()}
                className="w-full bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded transition-colors"
              >
                Reload Page
              </button>
              <button
                onClick={() => this.setState({ hasError: false, error: null, errorInfo: null })}
                className="w-full bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded transition-colors"
              >
                Continue Anyway
              </button>
            </div>
            {process.env.NODE_ENV === 'development' && this.state.error && (
              <details className="mt-4">
                <summary className="cursor-pointer text-sm text-gray-400">
                  Error Details (Development)
                </summary>
                <pre className="mt-2 text-xs text-red-300 overflow-auto max-h-32">
                  {this.state.error.toString()}
                  {this.state.errorInfo?.componentStack}
                </pre>
              </details>
            )}
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

// Toast Notification Component
export function ToastNotifications() {
  const { errors, removeError } = useToastStore();

  const getToastStyles = (type: ErrorInfo['type']) => {
    switch (type) {
      case 'error':
        return 'bg-red-600 border-red-500';
      case 'warning':
        return 'bg-yellow-600 border-yellow-500';
      case 'info':
        return 'bg-blue-600 border-blue-500';
      case 'success':
        return 'bg-green-600 border-green-500';
      default:
        return 'bg-gray-600 border-gray-500';
    }
  };

  const getIcon = (type: ErrorInfo['type']) => {
    switch (type) {
      case 'error':
        return '❌';
      case 'warning':
        return '⚠️';
      case 'info':
        return 'ℹ️';
      case 'success':
        return '✅';
      default:
        return '📢';
    }
  };

  return (
    <div className="fixed top-4 right-4 z-50 space-y-2 max-w-sm">
      {errors.map((error) => (
        <div
          key={error.id}
          className={`
            ${getToastStyles(error.type)}
            border rounded-lg p-4 text-white shadow-lg transform transition-all duration-300
            animate-in slide-in-from-right
          `}
        >
          <div className="flex items-start justify-between">
            <div className="flex items-start space-x-2 flex-1">
              <span className="text-lg">{getIcon(error.type)}</span>
              <div className="flex-1">
                <p className="font-medium">{error.message}</p>
                {error.details && (
                  <details className="mt-1">
                    <summary className="text-xs opacity-75 cursor-pointer">
                      Details
                    </summary>
                    <pre className="mt-1 text-xs opacity-75 whitespace-pre-wrap">
                      {typeof error.details === 'string' 
                        ? error.details 
                        : JSON.stringify(error.details, null, 2)
                      }
                    </pre>
                  </details>
                )}
                <p className="text-xs opacity-75 mt-1">
                  {new Date(error.timestamp).toLocaleTimeString()} • {error.source}
                </p>
              </div>
            </div>
            <button
              onClick={() => removeError(error.id)}
              className="ml-2 text-white hover:text-gray-200 transition-colors"
              aria-label="Dismiss"
            >
              ✕
            </button>
          </div>
          {error.action && (
            <button
              onClick={error.action.handler}
              className="mt-2 w-full bg-white bg-opacity-20 hover:bg-opacity-30 px-3 py-1 rounded text-sm transition-colors"
            >
              {error.action.label}
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

// Hook for easy error reporting
export function useErrorReporting() {
  const { addError } = useToastStore();

  const reportError = useCallback((
    message: string,
    type: ErrorInfo['type'] = 'error',
    source: ErrorInfo['source'] = 'system',
    details?: any,
    action?: ErrorInfo['action']
  ) => {
    addError({
      message,
      type,
      source,
      details,
      action,
    });
  }, [addError]);

  const reportSuccess = useCallback((message: string, details?: any) => {
    reportError(message, 'success', 'system', details);
  }, [reportError]);

  const reportWarning = useCallback((message: string, source: ErrorInfo['source'] = 'system', details?: any) => {
    reportError(message, 'warning', source, details);
  }, [reportError]);

  const reportInfo = useCallback((message: string, details?: any) => {
    reportError(message, 'info', 'system', details);
  }, [reportError]);

  // Audio-specific error reporters
  const reportAudioError = useCallback((message: string, details?: any) => {
    reportError(message, 'error', 'audio', details, {
      label: 'Test Audio',
      handler: () => {
        // Trigger audio test
        console.log('Testing audio system...');
      },
    });
  }, [reportError]);

  const reportMidiError = useCallback((message: string, details?: any) => {
    reportError(message, 'error', 'midi', details, {
      label: 'Check MIDI',
      handler: () => {
        // Open MIDI settings
        console.log('Opening MIDI settings...');
      },
    });
  }, [reportError]);

  return {
    reportError,
    reportSuccess,
    reportWarning,
    reportInfo,
    reportAudioError,
    reportMidiError,
  };
}

// Higher-order component for error handling
export function withErrorBoundary<P extends object>(
  Component: React.ComponentType<P>,
  fallback?: ReactNode,
  onError?: (error: Error, errorInfo: ErrorInfo) => void
) {
  return function WrappedComponent(props: P) {
    return (
      <ErrorBoundary fallback={fallback} onError={onError}>
        <Component {...props} />
      </ErrorBoundary>
    );
  };
}

// Development error overlay (only in development)
export function DevelopmentErrorOverlay() {
  const { errors } = useToastStore();
  const errorCount = errors.filter(e => e.type === 'error').length;
  
  if (process.env.NODE_ENV !== 'development' || errorCount === 0) {
    return null;
  }

  return (
    <div className="fixed bottom-4 left-4 bg-red-600 text-white px-3 py-2 rounded-lg text-sm z-40">
      🐛 {errorCount} error{errorCount !== 1 ? 's' : ''} in development
    </div>
  );
}