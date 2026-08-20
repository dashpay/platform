import * as sdk from '../../../dist/sdk.compressed.js';

const DEFAULT_TIMEOUT_MS = 60_000;
const DEFAULT_INTERVAL_MS = 2_000;

export interface PrefetchLocalReadyOptions {
  timeoutMs?: number;
  intervalMs?: number;
}

export interface PrefetchLocalReadyDependencies {
  prefetchLocal?: () => Promise<sdk.WasmTrustedContext>;
  now?: () => number;
  sleep?: (durationMs: number) => Promise<void>;
}

// Wait for the local dashmate network to be ready, then return a prefetched
// WasmTrustedContext. Retries on any error from prefetchLocal() until the
// timeout — masternodes can take time to reach status=ENABLED + versionCheck=success
// after `yarn start`, and a single failed attempt would otherwise abort the suite.
export async function prefetchLocalReady(
  options: PrefetchLocalReadyOptions = {},
  dependencies: PrefetchLocalReadyDependencies = {},
): Promise<sdk.WasmTrustedContext> {
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const intervalMs = options.intervalMs ?? DEFAULT_INTERVAL_MS;
  const prefetchLocal = dependencies.prefetchLocal
    ?? (() => sdk.WasmTrustedContext.prefetchLocal());
  const now = dependencies.now ?? (() => performance.now());
  const sleep = dependencies.sleep ?? ((durationMs: number) => (
    new Promise((resolve) => {
      setTimeout(resolve, durationMs);
    })
  ));
  const deadline = now() + timeoutMs;
  let lastError: unknown;
  let attempts = 0;

  while (now() < deadline) {
    attempts += 1;
    try {
      return await prefetchLocal();
    } catch (error) {
      lastError = error;
      const remaining = deadline - now();
      if (remaining > 0) {
        await sleep(Math.min(intervalMs, remaining));
      }
    }
  }

  let message: string;
  if (lastError instanceof Error) {
    message = lastError.message;
  } else if (lastError && typeof lastError === 'object' && 'message' in lastError) {
    message = String(lastError.message);
  } else {
    message = String(lastError);
  }

  throw new Error(
    `prefetchLocalReady: local network not ready after ${timeoutMs}ms (${attempts} attempts); last error: ${message}`,
  );
}
