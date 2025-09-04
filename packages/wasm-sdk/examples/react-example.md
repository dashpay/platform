# React Integration Example

This guide demonstrates how to integrate the Dash Platform WASM SDK into a React application using the modern WasmSDK wrapper.

## Table of Contents

- [Installation](#installation)
- [Basic Hook Implementation](#basic-hook-implementation)
- [Context Provider Pattern](#context-provider-pattern)
- [Component Examples](#component-examples)
- [Error Handling](#error-handling)
- [Performance Optimization](#performance-optimization)
- [Complete Dashboard Example](#complete-dashboard-example)
- [TypeScript Support](#typescript-support)
- [Troubleshooting](#troubleshooting)

## Installation

```bash
npm install @dashevo/dash-wasm-sdk react react-dom

# For TypeScript projects
npm install --save-dev @types/react @types/react-dom
```

## Basic Hook Implementation

### Simple useWasmSDK Hook

```javascript
// hooks/useWasmSDK.js
import { useState, useEffect, useRef } from 'react';
import { WasmSDK } from '@dashevo/dash-wasm-sdk';

export function useWasmSDK(config) {
    const [sdk, setSdk] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const sdkRef = useRef(null);

    useEffect(() => {
        let isMounted = true;
        
        async function initializeSDK() {
            try {
                setLoading(true);
                setError(null);
                
                const newSdk = new WasmSDK(config);
                await newSdk.initialize();
                
                if (isMounted) {
                    sdkRef.current = newSdk;
                    setSdk(newSdk);
                    setLoading(false);
                }
            } catch (err) {
                if (isMounted) {
                    setError(err);
                    setLoading(false);
                }
            }
        }

        initializeSDK();

        return () => {
            isMounted = false;
            if (sdkRef.current) {
                sdkRef.current.destroy();
            }
        };
    }, [JSON.stringify(config)]);

    return { sdk, loading, error };
}
```

### Advanced Hook with Retry Logic

```javascript
// hooks/useWasmSDKAdvanced.js
import { useState, useEffect, useRef, useCallback } from 'react';
import { WasmSDK, WasmTransportError } from '@dashevo/dash-wasm-sdk';

export function useWasmSDKAdvanced(config, options = {}) {
    const {
        maxRetries = 3,
        retryDelay = 1000,
        enableResourceCleanup = true,
        cleanupInterval = 300000
    } = options;

    const [sdk, setSdk] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [retryCount, setRetryCount] = useState(0);
    const sdkRef = useRef(null);
    const cleanupIntervalRef = useRef(null);

    const initializeSDK = useCallback(async (attempt = 1) => {
        try {
            setLoading(true);
            setError(null);
            
            const newSdk = new WasmSDK(config);
            await newSdk.initialize();
            
            sdkRef.current = newSdk;
            setSdk(newSdk);
            setLoading(false);
            setRetryCount(0);

            // Set up periodic resource cleanup
            if (enableResourceCleanup) {
                cleanupIntervalRef.current = setInterval(() => {
                    if (sdkRef.current) {
                        sdkRef.current.cleanupResources();
                    }
                }, cleanupInterval);
            }
        } catch (err) {
            if (err instanceof WasmTransportError && attempt < maxRetries) {
                setRetryCount(attempt);
                setTimeout(() => {
                    initializeSDK(attempt + 1);
                }, retryDelay * attempt);
            } else {
                setError(err);
                setLoading(false);
            }
        }
    }, [config, maxRetries, retryDelay, enableResourceCleanup, cleanupInterval]);

    useEffect(() => {
        initializeSDK();

        return () => {
            if (cleanupIntervalRef.current) {
                clearInterval(cleanupIntervalRef.current);
            }
            if (sdkRef.current) {
                sdkRef.current.destroy();
            }
        };
    }, [initializeSDK]);

    const retry = useCallback(() => {
        initializeSDK();
    }, [initializeSDK]);

    return { 
        sdk, 
        loading, 
        error, 
        retryCount, 
        maxRetries, 
        retry 
    };
}
```

## Context Provider Pattern

### SDK Context Provider

```javascript
// context/WasmSDKContext.js
import React, { createContext, useContext } from 'react';
import { useWasmSDKAdvanced } from '../hooks/useWasmSDKAdvanced';

const WasmSDKContext = createContext(null);

export function WasmSDKProvider({ children, config, options }) {
    const sdkState = useWasmSDKAdvanced(config, options);

    return (
        <WasmSDKContext.Provider value={sdkState}>
            {children}
        </WasmSDKContext.Provider>
    );
}

export function useWasmSDKContext() {
    const context = useContext(WasmSDKContext);
    if (!context) {
        throw new Error('useWasmSDKContext must be used within a WasmSDKProvider');
    }
    return context;
}
```

### App Setup with Context

```javascript
// App.js
import React from 'react';
import { WasmSDKProvider } from './context/WasmSDKContext';
import Dashboard from './components/Dashboard';
import ErrorBoundary from './components/ErrorBoundary';

const sdkConfig = {
    network: 'testnet',
    transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000,
        retries: 3
    },
    proofs: true,
    debug: process.env.NODE_ENV === 'development'
};

const sdkOptions = {
    maxRetries: 3,
    retryDelay: 1000,
    enableResourceCleanup: true
};

function App() {
    return (
        <ErrorBoundary>
            <WasmSDKProvider config={sdkConfig} options={sdkOptions}>
                <div className="App">
                    <header className="App-header">
                        <h1>Dash Platform React App</h1>
                    </header>
                    <main>
                        <Dashboard />
                    </main>
                </div>
            </WasmSDKProvider>
        </ErrorBoundary>
    );
}

export default App;
```

## Component Examples

### Identity Display Component

```javascript
// components/IdentityDisplay.js
import React, { useState, useEffect } from 'react';
import { useWasmSDKContext } from '../context/WasmSDKContext';

function IdentityDisplay({ identityId }) {
    const { sdk, loading: sdkLoading } = useWasmSDKContext();
    const [identity, setIdentity] = useState(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState(null);

    useEffect(() => {
        if (!sdk || !identityId || sdkLoading) return;

        async function fetchIdentity() {
            try {
                setLoading(true);
                setError(null);
                const result = await sdk.getIdentity(identityId);
                setIdentity(result);
            } catch (err) {
                setError(err);
            } finally {
                setLoading(false);
            }
        }

        fetchIdentity();
    }, [sdk, identityId, sdkLoading]);

    if (sdkLoading) {
        return <div className="loading">Initializing SDK...</div>;
    }

    if (loading) {
        return <div className="loading">Loading identity...</div>;
    }

    if (error) {
        return (
            <div className="error">
                <h3>Error loading identity</h3>
                <p>{error.message}</p>
            </div>
        );
    }

    if (!identity) {
        return <div className="not-found">Identity not found</div>;
    }

    return (
        <div className="identity-display">
            <h3>Identity Details</h3>
            <p><strong>ID:</strong> {identityId}</p>
            <p><strong>Public Keys:</strong> {identity.publicKeys?.length || 0}</p>
            <p><strong>Balance:</strong> {identity.balance || 0} credits</p>
            <p><strong>Revision:</strong> {identity.revision}</p>
        </div>
    );
}

export default IdentityDisplay;
```

### Document List Component

```javascript
// components/DocumentList.js
import React, { useState, useEffect, useCallback } from 'react';
import { useWasmSDKContext } from '../context/WasmSDKContext';

function DocumentList({ contractId, documentType, ownerId }) {
    const { sdk, loading: sdkLoading } = useWasmSDKContext();
    const [documents, setDocuments] = useState([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState(null);
    const [page, setPage] = useState(0);
    const [hasMore, setHasMore] = useState(true);
    
    const limit = 10;

    const fetchDocuments = useCallback(async (pageNum = 0, append = false) => {
        if (!sdk || sdkLoading) return;

        try {
            setLoading(true);
            setError(null);

            const query = {
                limit,
                offset: pageNum * limit
            };

            if (ownerId) {
                query.where = [['ownerId', '=', ownerId]];
            }

            const results = await sdk.getDocuments(contractId, documentType, query);
            
            if (append) {
                setDocuments(prev => [...prev, ...results]);
            } else {
                setDocuments(results);
            }
            
            setHasMore(results.length === limit);
            setPage(pageNum);
        } catch (err) {
            setError(err);
        } finally {
            setLoading(false);
        }
    }, [sdk, sdkLoading, contractId, documentType, ownerId, limit]);

    useEffect(() => {
        fetchDocuments(0, false);
    }, [fetchDocuments]);

    const loadMore = () => {
        if (!loading && hasMore) {
            fetchDocuments(page + 1, true);
        }
    };

    const refresh = () => {
        fetchDocuments(0, false);
    };

    if (sdkLoading) {
        return <div className="loading">Initializing SDK...</div>;
    }

    return (
        <div className="document-list">
            <div className="document-list-header">
                <h3>Documents ({documentType})</h3>
                <button onClick={refresh} disabled={loading}>
                    Refresh
                </button>
            </div>

            {error && (
                <div className="error">
                    <p>Error: {error.message}</p>
                    <button onClick={refresh}>Retry</button>
                </div>
            )}

            <div className="documents">
                {documents.map((doc, index) => (
                    <div key={`${doc.id || index}`} className="document-item">
                        <h4>Document #{index + 1}</h4>
                        <p><strong>ID:</strong> {doc.id}</p>
                        <p><strong>Owner:</strong> {doc.ownerId}</p>
                        <pre>{JSON.stringify(doc.data, null, 2)}</pre>
                    </div>
                ))}
            </div>

            {hasMore && (
                <button 
                    onClick={loadMore} 
                    disabled={loading}
                    className="load-more"
                >
                    {loading ? 'Loading...' : 'Load More'}
                </button>
            )}

            {documents.length === 0 && !loading && (
                <div className="no-documents">
                    No documents found
                </div>
            )}
        </div>
    );
}

export default DocumentList;
```

## Error Handling

### Error Boundary Component

```javascript
// components/ErrorBoundary.js
import React from 'react';
import { WasmSDKError } from '@dashevo/dash-wasm-sdk';

class ErrorBoundary extends React.Component {
    constructor(props) {
        super(props);
        this.state = { hasError: false, error: null, errorInfo: null };
    }

    static getDerivedStateFromError(error) {
        return { hasError: true };
    }

    componentDidCatch(error, errorInfo) {
        this.setState({
            error,
            errorInfo
        });

        // Log error details
        console.error('React Error Boundary caught an error:', error, errorInfo);
        
        // You could send this to an error reporting service
        if (error instanceof WasmSDKError) {
            console.error('WASM SDK Error details:', {
                code: error.code,
                context: error.context,
                operation: error.operation
            });
        }
    }

    render() {
        if (this.state.hasError) {
            return (
                <div className="error-boundary">
                    <h2>Something went wrong</h2>
                    <details style={{ whiteSpace: 'pre-wrap' }}>
                        <summary>Error Details</summary>
                        <p><strong>Error:</strong> {this.state.error && this.state.error.toString()}</p>
                        <p><strong>Stack trace:</strong> {this.state.errorInfo.componentStack}</p>
                    </details>
                    <button onClick={() => window.location.reload()}>
                        Refresh Page
                    </button>
                </div>
            );
        }

        return this.props.children;
    }
}

export default ErrorBoundary;
```

### Error Display Hook

```javascript
// hooks/useErrorHandler.js
import { useState, useCallback } from 'react';
import { 
    WasmTransportError, 
    WasmOperationError, 
    WasmInitializationError 
} from '@dashevo/dash-wasm-sdk';

export function useErrorHandler() {
    const [error, setError] = useState(null);

    const handleError = useCallback((err) => {
        console.error('Error occurred:', err);

        let userMessage = 'An unexpected error occurred';
        let canRetry = false;

        if (err instanceof WasmTransportError) {
            userMessage = 'Network connection error. Please check your internet connection.';
            canRetry = true;
        } else if (err instanceof WasmInitializationError) {
            userMessage = 'Failed to initialize SDK. Please refresh the page.';
            canRetry = true;
        } else if (err instanceof WasmOperationError) {
            userMessage = `Operation failed: ${err.operation}`;
            canRetry = err.operation !== 'validation';
        }

        setError({
            original: err,
            message: userMessage,
            canRetry,
            timestamp: new Date()
        });
    }, []);

    const clearError = useCallback(() => {
        setError(null);
    }, []);

    return { error, handleError, clearError };
}
```

## Performance Optimization

### Memoization Example

```javascript
// components/OptimizedIdentityDisplay.js
import React, { useMemo, memo } from 'react';
import { useWasmSDKContext } from '../context/WasmSDKContext';
import { useAsyncMemo } from '../hooks/useAsyncMemo';

const OptimizedIdentityDisplay = memo(({ identityId }) => {
    const { sdk, loading: sdkLoading } = useWasmSDKContext();

    // Memoize the async operation
    const { data: identity, loading, error } = useAsyncMemo(
        async () => {
            if (!sdk || !identityId) return null;
            return await sdk.getIdentity(identityId);
        },
        [sdk, identityId],
        { cacheTime: 300000 } // Cache for 5 minutes
    );

    // Memoize expensive calculations
    const identityStats = useMemo(() => {
        if (!identity) return null;
        
        return {
            keyCount: identity.publicKeys?.length || 0,
            balanceInDash: (identity.balance || 0) / 100000000,
            hasMultipleKeys: (identity.publicKeys?.length || 0) > 1
        };
    }, [identity]);

    if (sdkLoading || loading) {
        return <div className="loading">Loading...</div>;
    }

    if (error) {
        return <div className="error">Error: {error.message}</div>;
    }

    if (!identity) {
        return <div className="not-found">Identity not found</div>;
    }

    return (
        <div className="identity-display">
            <h3>Identity {identityId}</h3>
            <p>Keys: {identityStats.keyCount}</p>
            <p>Balance: {identityStats.balanceInDash} DASH</p>
            {identityStats.hasMultipleKeys && (
                <p className="warning">Multiple keys detected</p>
            )}
        </div>
    );
});

OptimizedIdentityDisplay.displayName = 'OptimizedIdentityDisplay';
export default OptimizedIdentityDisplay;
```

### Async Memo Hook

```javascript
// hooks/useAsyncMemo.js
import { useState, useEffect, useRef } from 'react';

export function useAsyncMemo(asyncFn, deps, options = {}) {
    const { cacheTime = 0 } = options;
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState(null);
    const cacheRef = useRef(null);
    const timestampRef = useRef(0);

    useEffect(() => {
        let isMounted = true;

        async function execute() {
            // Check cache
            if (cacheTime > 0 && cacheRef.current && 
                Date.now() - timestampRef.current < cacheTime) {
                setData(cacheRef.current);
                return;
            }

            try {
                setLoading(true);
                setError(null);
                const result = await asyncFn();
                
                if (isMounted) {
                    setData(result);
                    if (cacheTime > 0) {
                        cacheRef.current = result;
                        timestampRef.current = Date.now();
                    }
                }
            } catch (err) {
                if (isMounted) {
                    setError(err);
                }
            } finally {
                if (isMounted) {
                    setLoading(false);
                }
            }
        }

        execute();

        return () => {
            isMounted = false;
        };
    }, deps);

    return { data, loading, error };
}
```

## Complete Dashboard Example

```javascript
// components/Dashboard.js
import React, { useState } from 'react';
import { useWasmSDKContext } from '../context/WasmSDKContext';
import IdentityDisplay from './IdentityDisplay';
import DocumentList from './DocumentList';
import { useErrorHandler } from '../hooks/useErrorHandler';

function Dashboard() {
    const { sdk, loading, error: sdkError, retry } = useWasmSDKContext();
    const { error, handleError, clearError } = useErrorHandler();
    const [identityId, setIdentityId] = useState('');
    const [contractId, setContractId] = useState('');
    const [activeTab, setActiveTab] = useState('identity');

    if (loading) {
        return (
            <div className="dashboard loading">
                <h2>Initializing Dash Platform SDK...</h2>
                <div className="spinner"></div>
            </div>
        );
    }

    if (sdkError) {
        return (
            <div className="dashboard error">
                <h2>SDK Initialization Failed</h2>
                <p>{sdkError.message}</p>
                <button onClick={retry}>Retry</button>
            </div>
        );
    }

    return (
        <div className="dashboard">
            <div className="dashboard-header">
                <h2>Dash Platform Dashboard</h2>
                <div className="network-info">
                    Network: {sdk?.getNetwork()}
                </div>
            </div>

            {error && (
                <div className="error-banner">
                    <p>{error.message}</p>
                    <button onClick={clearError}>Dismiss</button>
                </div>
            )}

            <div className="dashboard-controls">
                <div className="input-group">
                    <label>
                        Identity ID:
                        <input 
                            type="text"
                            value={identityId}
                            onChange={(e) => setIdentityId(e.target.value)}
                            placeholder="Enter identity ID..."
                        />
                    </label>
                </div>

                <div className="input-group">
                    <label>
                        Contract ID:
                        <input 
                            type="text"
                            value={contractId}
                            onChange={(e) => setContractId(e.target.value)}
                            placeholder="Enter contract ID..."
                        />
                    </label>
                </div>
            </div>

            <div className="dashboard-tabs">
                <button 
                    className={activeTab === 'identity' ? 'active' : ''}
                    onClick={() => setActiveTab('identity')}
                >
                    Identity
                </button>
                <button 
                    className={activeTab === 'documents' ? 'active' : ''}
                    onClick={() => setActiveTab('documents')}
                >
                    Documents
                </button>
            </div>

            <div className="dashboard-content">
                {activeTab === 'identity' && identityId && (
                    <IdentityDisplay identityId={identityId} />
                )}

                {activeTab === 'documents' && contractId && (
                    <DocumentList 
                        contractId={contractId}
                        documentType="note"
                        ownerId={identityId || null}
                    />
                )}

                {!identityId && activeTab === 'identity' && (
                    <div className="placeholder">
                        Enter an Identity ID to view details
                    </div>
                )}

                {!contractId && activeTab === 'documents' && (
                    <div className="placeholder">
                        Enter a Contract ID to view documents
                    </div>
                )}
            </div>
        </div>
    );
}

export default Dashboard;
```

## TypeScript Support

### Type Definitions

```typescript
// types/sdk.ts
import { WasmSDK, WasmSDKConfig } from '@dashevo/dash-wasm-sdk';

export interface UseWasmSDKResult {
    sdk: WasmSDK | null;
    loading: boolean;
    error: Error | null;
    retryCount?: number;
    maxRetries?: number;
    retry?: () => void;
}

export interface WasmSDKContextValue extends UseWasmSDKResult {}

export interface WasmSDKProviderProps {
    children: React.ReactNode;
    config: WasmSDKConfig;
    options?: {
        maxRetries?: number;
        retryDelay?: number;
        enableResourceCleanup?: boolean;
        cleanupInterval?: number;
    };
}
```

### TypeScript Component Example

```typescript
// components/TypedIdentityDisplay.tsx
import React, { useState, useEffect } from 'react';
import { useWasmSDKContext } from '../context/WasmSDKContext';
import { WasmOperationError } from '@dashevo/dash-wasm-sdk';

interface Identity {
    id: string;
    publicKeys: Array<{
        id: number;
        type: number;
        data: Uint8Array;
    }>;
    balance: number;
    revision: number;
}

interface IdentityDisplayProps {
    identityId: string;
    onError?: (error: Error) => void;
}

const TypedIdentityDisplay: React.FC<IdentityDisplayProps> = ({ 
    identityId, 
    onError 
}) => {
    const { sdk, loading: sdkLoading } = useWasmSDKContext();
    const [identity, setIdentity] = useState<Identity | null>(null);
    const [loading, setLoading] = useState<boolean>(false);
    const [error, setError] = useState<Error | null>(null);

    useEffect(() => {
        if (!sdk || !identityId || sdkLoading) return;

        async function fetchIdentity(): Promise<void> {
            try {
                setLoading(true);
                setError(null);
                const result = await sdk.getIdentity(identityId);
                setIdentity(result as Identity);
            } catch (err) {
                const error = err as Error;
                setError(error);
                onError?.(error);
                
                if (err instanceof WasmOperationError) {
                    console.error('Operation failed:', err.operation, err.context);
                }
            } finally {
                setLoading(false);
            }
        }

        fetchIdentity();
    }, [sdk, identityId, sdkLoading, onError]);

    if (sdkLoading) {
        return <div className="loading">Initializing SDK...</div>;
    }

    if (loading) {
        return <div className="loading">Loading identity...</div>;
    }

    if (error) {
        return (
            <div className="error">
                <h3>Error loading identity</h3>
                <p>{error.message}</p>
            </div>
        );
    }

    if (!identity) {
        return <div className="not-found">Identity not found</div>;
    }

    return (
        <div className="identity-display">
            <h3>Identity Details</h3>
            <p><strong>ID:</strong> {identity.id}</p>
            <p><strong>Public Keys:</strong> {identity.publicKeys.length}</p>
            <p><strong>Balance:</strong> {identity.balance} credits</p>
            <p><strong>Revision:</strong> {identity.revision}</p>
        </div>
    );
};

export default TypedIdentityDisplay;
```

## Troubleshooting

### Common React-Specific Issues

#### 1. Hook Dependency Array Issues

```javascript
// Problem: Infinite re-renders due to object reference changes
useEffect(() => {
    // This will cause infinite loop if config is an object
    initializeSDK();
}, [config]); 

// Solution: Stringify or use deep comparison
useEffect(() => {
    initializeSDK();
}, [JSON.stringify(config)]);
```

#### 2. Memory Leaks in Components

```javascript
// Problem: SDK not cleaned up when component unmounts
useEffect(() => {
    const sdk = new WasmSDK(config);
    sdk.initialize();
    // Missing cleanup!
}, []);

// Solution: Always cleanup in useEffect return
useEffect(() => {
    const sdk = new WasmSDK(config);
    sdk.initialize();
    
    return () => {
        sdk.destroy(); // Always cleanup
    };
}, []);
```

#### 3. State Updates After Unmount

```javascript
// Problem: Setting state after component unmounts
useEffect(() => {
    async function fetchData() {
        const data = await sdk.getIdentity(id);
        setData(data); // Could run after unmount!
    }
    fetchData();
}, []);

// Solution: Use cleanup flag
useEffect(() => {
    let isMounted = true;
    
    async function fetchData() {
        const data = await sdk.getIdentity(id);
        if (isMounted) {
            setData(data);
        }
    }
    fetchData();
    
    return () => {
        isMounted = false;
    };
}, []);
```

#### 4. Context Value Stability

```javascript
// Problem: Context value changes on every render
function MyProvider({ children }) {
    const value = { sdk, loading, error }; // New object every render!
    return <Context.Provider value={value}>{children}</Context.Provider>;
}

// Solution: Use useMemo to stabilize value
function MyProvider({ children }) {
    const value = useMemo(() => ({ sdk, loading, error }), [sdk, loading, error]);
    return <Context.Provider value={value}>{children}</Context.Provider>;
}
```

### Development vs Production

```javascript
// Different configurations for different environments
const sdkConfig = {
    network: process.env.REACT_APP_NETWORK || 'testnet',
    transport: {
        url: process.env.REACT_APP_SDK_URL || 'https://52.12.176.90:1443/',
        timeout: process.env.NODE_ENV === 'development' ? 60000 : 30000
    },
    debug: process.env.NODE_ENV === 'development'
};
```

### Error Recovery

```javascript
// Component with error recovery
function ResilientComponent() {
    const [key, setKey] = useState(0);
    
    const handleError = useCallback(() => {
        // Force component remount to recover from errors
        setKey(prev => prev + 1);
    }, []);
    
    return (
        <ErrorBoundary key={key} onError={handleError}>
            <MySDKComponent />
        </ErrorBoundary>
    );
}
```

This React integration provides a robust foundation for building Dash Platform applications with proper error handling, performance optimization, and TypeScript support.