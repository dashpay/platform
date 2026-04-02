# Tutorial: React Integration

Build a React application that connects to Dash Platform, queries data, and
broadcasts state transitions. This tutorial covers SDK initialization in a
React context, handling async WASM loading, and patterns for queries and
mutations.

## What you will learn

- Initializing the Evo SDK in a React app with proper lifecycle management
- Creating a React context/provider for SDK access
- Building hooks for queries and state transitions
- Handling loading, error, and connected states
- Working with the SDK in both development and production builds

## Prerequisites

```sh
npx create-vite@latest my-dash-app -- --template react-ts
cd my-dash-app
npm install @dashevo/evo-sdk
```

> **Vite** is recommended because it handles WASM imports natively. Create
> React App (webpack 4) requires additional configuration for WASM — Vite
> works out of the box.

## Step 1: Create the SDK provider

The SDK must be initialized once and shared across the app. A React context is
the natural fit.

**`src/DashProvider.tsx`**

```tsx
import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { EvoSDK } from '@dashevo/evo-sdk';

interface DashContextValue {
  sdk: EvoSDK | null;
  isConnecting: boolean;
  error: string | null;
}

const DashContext = createContext<DashContextValue>({
  sdk: null,
  isConnecting: true,
  error: null,
});

export function useDash() {
  return useContext(DashContext);
}

export function useSDK(): EvoSDK {
  const { sdk } = useDash();
  if (!sdk) throw new Error('SDK not connected. Wrap your app in <DashProvider>.');
  return sdk;
}

interface DashProviderProps {
  network?: 'testnet' | 'mainnet' | 'local';
  children: ReactNode;
}

export function DashProvider({ network = 'testnet', children }: DashProviderProps) {
  const [sdk, setSdk] = useState<EvoSDK | null>(null);
  const [isConnecting, setIsConnecting] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function connect() {
      try {
        setIsConnecting(true);
        setError(null);

        const instance = new EvoSDK({ network, trusted: true });
        await instance.connect();

        if (!cancelled) {
          setSdk(instance);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to connect');
        }
      } finally {
        if (!cancelled) {
          setIsConnecting(false);
        }
      }
    }

    connect();

    return () => {
      cancelled = true;
    };
  }, [network]);

  return (
    <DashContext.Provider value={{ sdk, isConnecting, error }}>
      {children}
    </DashContext.Provider>
  );
}
```

**`src/main.tsx`**

```tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { DashProvider } from './DashProvider';
import App from './App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <DashProvider network="testnet">
      <App />
    </DashProvider>
  </StrictMode>,
);
```

## Step 2: Build query hooks

Create reusable hooks for common queries. These handle loading and error states
automatically.

**`src/hooks/useDashQuery.ts`**

```ts
import { useEffect, useState } from 'react';
import { useDash } from '../DashProvider';
import type { EvoSDK } from '@dashevo/evo-sdk';

interface QueryState<T> {
  data: T | null;
  isLoading: boolean;
  error: string | null;
  refetch: () => void;
}

export function useDashQuery<T>(
  queryFn: (sdk: EvoSDK) => Promise<T>,
  deps: unknown[] = [],
): QueryState<T> {
  const { sdk } = useDash();
  const [data, setData] = useState<T | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [trigger, setTrigger] = useState(0);

  useEffect(() => {
    if (!sdk) return;

    let cancelled = false;
    setIsLoading(true);

    queryFn(sdk)
      .then((result) => {
        if (!cancelled) setData(result);
      })
      .catch((err) => {
        if (!cancelled) setError(err.message);
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => { cancelled = true; };
  }, [sdk, trigger, ...deps]);

  return {
    data,
    isLoading,
    error,
    refetch: () => setTrigger((n) => n + 1),
  };
}
```

### Specific query hooks

**`src/hooks/useIdentity.ts`**

```ts
import { useDashQuery } from './useDashQuery';

export function useIdentity(identityId: string) {
  return useDashQuery(
    (sdk) => sdk.identities.fetch(identityId),
    [identityId],
  );
}
```

**`src/hooks/useDocuments.ts`**

```ts
import { useDashQuery } from './useDashQuery';
import type { DocumentsQuery } from '@dashevo/evo-sdk';

export function useDocuments(query: DocumentsQuery) {
  return useDashQuery(
    (sdk) => sdk.documents.query(query),
    [JSON.stringify(query)],
  );
}
```

**`src/hooks/useTokenBalance.ts`**

```ts
import { useDashQuery } from './useDashQuery';

export function useTokenBalance(identityId: string, tokenId: string) {
  return useDashQuery(
    async (sdk) => {
      const balances = await sdk.tokens.identityBalances(identityId, [tokenId]);
      for (const [id, balance] of balances) {
        if (id.toString() === tokenId) return balance;
      }
      return 0n;
    },
    [identityId, tokenId],
  );
}
```

## Step 3: Build a mutation hook

For state transitions (writes), create a hook that manages submission state:

**`src/hooks/useDashMutation.ts`**

```ts
import { useState, useCallback } from 'react';
import { useSDK } from '../DashProvider';
import type { EvoSDK } from '@dashevo/evo-sdk';

interface MutationState<T> {
  execute: () => Promise<T | undefined>;
  isSubmitting: boolean;
  error: string | null;
  reset: () => void;
}

export function useDashMutation<T>(
  mutationFn: (sdk: EvoSDK) => Promise<T>,
): MutationState<T> {
  const sdk = useSDK();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(async () => {
    try {
      setIsSubmitting(true);
      setError(null);
      const result = await mutationFn(sdk);
      return result;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Transaction failed');
    } finally {
      setIsSubmitting(false);
    }
  }, [sdk, mutationFn]);

  return {
    execute,
    isSubmitting,
    error,
    reset: () => setError(null),
  };
}
```

## Step 4: Build components

### Connection status

**`src/components/ConnectionStatus.tsx`**

```tsx
import { useDash } from '../DashProvider';

export function ConnectionStatus() {
  const { sdk, isConnecting, error } = useDash();

  if (isConnecting) return <span className="status connecting">Connecting...</span>;
  if (error) return <span className="status error">Error: {error}</span>;
  if (sdk) return <span className="status connected">Connected to testnet</span>;
  return null;
}
```

### Identity viewer

**`src/components/IdentityViewer.tsx`**

```tsx
import { useState } from 'react';
import { useIdentity } from '../hooks/useIdentity';

export function IdentityViewer() {
  const [identityId, setIdentityId] = useState('');
  const [searchId, setSearchId] = useState('');
  const { data: identity, isLoading, error } = useIdentity(searchId);

  return (
    <div>
      <h2>Fetch Identity</h2>
      <form onSubmit={(e) => { e.preventDefault(); setSearchId(identityId); }}>
        <input
          value={identityId}
          onChange={(e) => setIdentityId(e.target.value)}
          placeholder="Enter identity ID"
        />
        <button type="submit">Fetch</button>
      </form>

      {isLoading && searchId && <p>Loading...</p>}
      {error && <p className="error">{error}</p>}
      {identity && (
        <div className="identity-card">
          <p><strong>ID:</strong> {identity.id.toString()}</p>
          <p><strong>Balance:</strong> {identity.balance.toString()} credits</p>
          <p><strong>Public keys:</strong> {identity.publicKeys.length}</p>
        </div>
      )}
    </div>
  );
}
```

### Document list (e.g., car listings from the previous tutorial)

**`src/components/ListingsList.tsx`**

```tsx
import { useDocuments } from '../hooks/useDocuments';

const CONTRACT_ID = 'YOUR_CONTRACT_ID';

export function ListingsList() {
  const { data: results, isLoading, error, refetch } = useDocuments({
    dataContractId: CONTRACT_ID,
    documentTypeName: 'listing',
    where: [['status', '==', 'available']],
    orderBy: [['priceUsd', 'asc']],
    limit: 20,
  });

  if (isLoading) return <p>Loading listings...</p>;
  if (error) return <p className="error">{error}</p>;
  if (!results || results.size === 0) return <p>No listings found.</p>;

  return (
    <div>
      <h2>Available Cars</h2>
      <button onClick={refetch}>Refresh</button>
      <ul>
        {[...results.entries()].map(([id, doc]) => {
          if (!doc) return null;
          const d = doc.properties as Record<string, unknown>;
          return (
            <li key={id}>
              <strong>{d.year} {d.make} {d.model}</strong> — ${d.priceUsd}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
```

### Create document form

**`src/components/CreateListing.tsx`**

```tsx
import { useState, useCallback } from 'react';
import { Document, Identifier, IdentitySigner } from '@dashevo/evo-sdk';
import { useDashMutation } from '../hooks/useDashMutation';

const CONTRACT_ID = 'YOUR_CONTRACT_ID';
const IDENTITY_ID = 'YOUR_IDENTITY_ID';
const PRIVATE_KEY = 'YOUR_PRIVATE_KEY_WIF';
const SIGNING_KEY_INDEX = 0;

export function CreateListing() {
  const [make, setMake] = useState('');
  const [model, setModel] = useState('');
  const [year, setYear] = useState(2024);
  const [price, setPrice] = useState(0);

  const mutation = useDashMutation(
    useCallback(
      async (sdk) => {
        const identity = await sdk.identities.fetch(IDENTITY_ID);
        const identityKey = identity.publicKeys[SIGNING_KEY_INDEX];
        const signer = new IdentitySigner();
        signer.addKeyFromWif(PRIVATE_KEY);

        const doc = new Document({
          documentTypeName: 'listing',
          dataContractId: new Identifier(CONTRACT_ID),
          ownerId: new Identifier(IDENTITY_ID),
          properties: {
            make,
            model,
            year,
            priceUsd: price,
            mileageKm: 0,
            status: 'available',
          },
        });
        return sdk.documents.create({ document: doc, identityKey, signer });
      },
      [make, model, year, price],
    ),
  );

  return (
    <form
      onSubmit={async (e) => {
        e.preventDefault();
        await mutation.execute();
      }}
    >
      <h2>Create Listing</h2>
      <input placeholder="Make" value={make} onChange={(e) => setMake(e.target.value)} />
      <input placeholder="Model" value={model} onChange={(e) => setModel(e.target.value)} />
      <input type="number" placeholder="Year" value={year} onChange={(e) => setYear(+e.target.value)} />
      <input type="number" placeholder="Price (USD)" value={price} onChange={(e) => setPrice(+e.target.value)} />
      <button type="submit" disabled={mutation.isSubmitting}>
        {mutation.isSubmitting ? 'Submitting...' : 'Create Listing'}
      </button>
      {mutation.error && <p className="error">{mutation.error}</p>}
    </form>
  );
}
```

## Step 5: Assemble the app

**`src/App.tsx`**

```tsx
import { ConnectionStatus } from './components/ConnectionStatus';
import { IdentityViewer } from './components/IdentityViewer';
import { ListingsList } from './components/ListingsList';
import { CreateListing } from './components/CreateListing';
import { useDash } from './DashProvider';

export default function App() {
  const { sdk } = useDash();

  return (
    <div className="app">
      <header>
        <h1>Dash Platform App</h1>
        <ConnectionStatus />
      </header>

      {sdk ? (
        <main>
          <IdentityViewer />
          <ListingsList />
          <CreateListing />
        </main>
      ) : (
        <p>Waiting for SDK connection...</p>
      )}
    </div>
  );
}
```

## Production considerations

### Private key management

The examples above hardcode private keys for clarity. In production:

- **Never** ship private keys in frontend code
- Use a backend service to sign state transitions, or
- Prompt the user for their mnemonic/key at runtime and keep it in memory only
- Consider the `wallet` namespace for key derivation from user-provided mnemonics

```tsx
import { wallet } from '@dashevo/evo-sdk';

async function signWithUserMnemonic(mnemonic: string) {
  const keyInfo = await wallet.deriveKeyFromSeedPhrase({
    mnemonic,
    network: 'testnet',
    derivationPath: "m/9'/1'/0'/0/0",
  });
  return keyInfo.privateKeyWif;
}
```

### Bundle size

The WASM module adds ~2-4 MB (gzipped) to your bundle. To optimise:

- Use **code splitting** — the SDK module only loads when `connect()` is called
- Vite handles WASM lazy loading automatically
- Consider loading the SDK only on pages that need it

### Error boundaries

Wrap SDK-dependent components in an error boundary to handle WASM
initialization failures gracefully:

```tsx
import { ErrorBoundary } from 'react-error-boundary';

<ErrorBoundary fallback={<p>Failed to load Dash SDK</p>}>
  <DashProvider>
    <App />
  </DashProvider>
</ErrorBoundary>
```

### Network switching

To let users switch networks at runtime, key the provider on the network value:

```tsx
const [network, setNetwork] = useState<'testnet' | 'mainnet'>('testnet');

<DashProvider network={network} key={network}>
  <App />
</DashProvider>
```

The `key` prop forces React to unmount and remount the provider, creating a
fresh SDK connection for the new network.
