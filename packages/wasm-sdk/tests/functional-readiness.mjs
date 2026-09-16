import init from '../dist/sdk.compressed.js';
import { prefetchLocalReady } from './functional/helpers/trustedContext.ts';

const NETWORK_READY_TIMEOUT_MS = 600_000;
const HOOK_TIMEOUT_MS = 630_000;

export const mochaHooks = {
  async beforeAll() {
    this.timeout(HOOK_TIMEOUT_MS);

    await init();
    const context = await prefetchLocalReady({
      timeoutMs: NETWORK_READY_TIMEOUT_MS,
    });
    context.free();
  },
};
