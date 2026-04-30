import * as sdk from '../../../dist/sdk.compressed.js';

const DEFAULT_TIMEOUT_MS = 60_000;
const DEFAULT_INTERVAL_MS = 2_000;

export interface PrefetchLocalReadyOptions {
  timeoutMs?: number;
  intervalMs?: number;
}

// Wait for the local dashmate network to be ready, then return a prefetched
// WasmTrustedContext. Retries on any error from prefetchLocal() until the
// timeout — masternodes can take time to reach status=ENABLED + versionCheck=success
// after `yarn start`, and a single failed attempt would otherwise abort the suite.
export async function prefetchLocalReady(
  options: PrefetchLocalReadyOptions = {},
): Promise<sdk.WasmTrustedContext> {
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const intervalMs = options.intervalMs ?? DEFAULT_INTERVAL_MS;
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;
  let attempts = 0;

  while (Date.now() < deadline) {
    attempts += 1;
    try {
      return await sdk.WasmTrustedContext.prefetchLocal();
    } catch (error) {
      lastError = error;
      const remaining = deadline - Date.now();
      if (remaining > intervalMs) {
        await new Promise((resolve) => {
          setTimeout(resolve, intervalMs);
        });
      }
    }
  }

  const message = lastError instanceof Error ? lastError.message : String(lastError);
  throw new Error(
    `prefetchLocalReady: local network not ready after ${timeoutMs}ms (${attempts} attempts); last error: ${message}`,
  );
}
