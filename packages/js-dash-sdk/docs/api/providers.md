# Providers API Reference

## WebServiceProvider

Connects to Dash quorum service endpoints for platform state and quorum keys.

### Constructor

```typescript
new WebServiceProvider(options?: WebServiceProviderOptions)
```

#### Options

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `network` | `'mainnet' \| 'testnet'` | `'testnet'` | Network to connect to |
| `url` | `string` | Network default | Custom service endpoint URL |
| `timeout` | `number` | `30000` | Request timeout in milliseconds |
| `retryAttempts` | `number` | `3` | Number of retry attempts on failure |
| `retryDelay` | `number` | `1000` | Initial retry delay in milliseconds |
| `cacheDuration` | `number` | `60000` | Cache duration in milliseconds |
| `headers` | `Record<string, string>` | `{}` | Custom HTTP headers |

### Methods

#### `getName(): string`
Returns the provider name.

#### `getCapabilities(): ProviderCapability[]`
Returns array of provider capabilities.

#### `isAvailable(): Promise<boolean>`
Checks if the service is available.

#### `getLatestPlatformBlockHeight(): Promise<number>`
Gets the latest platform block height.

#### `getLatestPlatformBlockTime(): Promise<number>`
Gets the latest platform block timestamp.

#### `getLatestPlatformCoreChainLockedHeight(): Promise<number>`
Gets the core chain locked height.

#### `getLatestPlatformVersion(): Promise<string>`
Gets the platform version.

#### `getQuorumKeys(): Promise<Map<string, QuorumInfo>>`
Fetches all quorum keys from the service.

#### `getQuorum(quorumHash: string): Promise<QuorumInfo | null>`
Gets a specific quorum by hash.

#### `getActiveQuorums(): Promise<QuorumInfo[]>`
Gets all active quorums.

#### `isValid(): Promise<boolean>`
Validates the provider connection.

## PriorityContextProvider

Manages multiple context providers with automatic fallback.

### Constructor

```typescript
new PriorityContextProvider(options: PriorityProviderOptions)
```

#### Options

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `providers` | `ProviderEntry[]` | Required | Array of provider configurations |
| `fallbackEnabled` | `boolean` | `true` | Enable automatic fallback |
| `cacheResults` | `boolean` | `false` | Cache successful responses |
| `logErrors` | `boolean` | `false` | Log provider errors to console |

#### ProviderEntry

| Property | Type | Description |
|----------|------|-------------|
| `provider` | `ContextProvider` | Provider instance |
| `priority` | `number` | Priority (higher = preferred) |
| `name` | `string` | Provider name for identification |
| `capabilities` | `ProviderCapability[]` | Optional capability override |

### Methods

#### `addProvider(provider: ContextProvider, priority: number, name: string): void`
Adds a provider at runtime.

#### `removeProvider(name: string): void`
Removes a provider by name.

#### `getActiveProvider(): Promise<ProviderEntry | null>`
Gets the highest priority available provider.

#### `getMetrics(): Map<string, ProviderMetrics>`
Returns performance metrics for all providers.

#### `clearCache(): void`
Clears the response cache.

### Events

#### `provider:used`
Emitted when a provider is used successfully.
```typescript
(name: string, method: string) => void
```

#### `provider:error`
Emitted when a provider fails.
```typescript
(name: string, error: Error) => void
```

#### `provider:fallback`
Emitted when fallback occurs.
```typescript
(from: string, to: string) => void
```

#### `all:failed`
Emitted when all providers fail.
```typescript
(method: string, errors: Map<string, Error>) => void
```

## ProviderFactory

Factory methods for creating configured providers.

### Static Methods

#### `create(options: ProviderFactoryOptions): Promise<ContextProvider>`
Creates a provider based on configuration.

```typescript
const provider = await ProviderFactory.create({
  providers: ['webservice', 'centralized'],
  network: 'testnet',
  usePriority: true
});
```

#### `createWithBluetooth(options: BluetoothFactoryOptions): Promise<ContextProvider>`
Creates a priority provider with Bluetooth as highest priority.

```typescript
const provider = await ProviderFactory.createWithBluetooth({
  network: 'testnet',
  bluetooth: {
    requireAuthentication: true
  },
  webservice: {
    cacheDuration: 60000
  }
});
```

#### `createWithWebService(options: WebServiceFactoryOptions): Promise<ContextProvider>`
Creates a priority provider with web service as highest priority.

```typescript
const provider = await ProviderFactory.createWithWebService({
  network: 'mainnet',
  webservice: {
    url: 'https://custom.service'
  }
});
```

## Types

### ProviderCapability

```typescript
enum ProviderCapability {
  PLATFORM_STATE = 'platform_state',
  QUORUM_KEYS = 'quorum_keys',
  BLOCK_HEADERS = 'block_headers',
  SUBSCRIPTIONS = 'subscriptions',
  TRANSACTION_SIGNING = 'transaction_signing'
}
```

### QuorumInfo

```typescript
interface QuorumInfo {
  quorumHash: string;
  quorumPublicKey: {
    version: number;
    publicKey: string;
    type: 'BLS' | 'ECDSA';
  };
  isActive: boolean;
}
```

### ProviderMetrics

```typescript
interface ProviderMetrics {
  successCount: number;
  errorCount: number;
  totalResponseTime: number;
  averageResponseTime: number;
  lastUsed: number;
  lastError?: Error;
}
```

### WebServiceProviderOptions

```typescript
interface WebServiceProviderOptions {
  network?: 'mainnet' | 'testnet';
  url?: string;
  timeout?: number;
  retryAttempts?: number;
  retryDelay?: number;
  cacheDuration?: number;
  headers?: Record<string, string>;
}
```

### PriorityProviderOptions

```typescript
interface PriorityProviderOptions {
  providers: ProviderEntry[];
  fallbackEnabled?: boolean;
  cacheResults?: boolean;
  logErrors?: boolean;
}
```

## Error Handling

### Provider Errors

All providers throw errors with these properties:

```typescript
class ProviderError extends Error {
  code: string;        // Error code
  provider: string;    // Provider name
  method: string;      // Method that failed
  cause?: Error;       // Original error
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `PROVIDER_UNAVAILABLE` | Provider cannot be reached |
| `INVALID_RESPONSE` | Invalid response format |
| `TIMEOUT` | Request timed out |
| `AUTHENTICATION_FAILED` | Authentication required |
| `ALL_PROVIDERS_FAILED` | All providers in priority list failed |

## Performance Considerations

### Caching

- Web service responses are cached to reduce network calls
- Cache duration is configurable per provider
- Priority provider can cache across all providers

### Retry Logic

- Exponential backoff with configurable attempts
- Only network errors are retried
- Client errors (4xx) fail immediately

### Timeout Handling

- All network calls have configurable timeouts
- Default timeout is 30 seconds
- Timeouts include retry attempts

## Security

### HTTPS

- All web service connections use HTTPS
- Certificate validation is enforced
- Custom certificates can be configured

### Authentication

- Custom headers support for API keys
- Bluetooth providers support device authentication
- No credentials are stored in the SDK