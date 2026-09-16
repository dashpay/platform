import { expect } from './helpers/chai.ts';
import { prefetchLocalReady } from '../functional/helpers/trustedContext.ts';

describe('prefetchLocalReady()', () => {
  it('should sleep through the deadline and report an object rejection message', async () => {
    const sleepDurations: number[] = [];
    const timestamps = [0, 0, 9, 10];
    let attempts = 0;
    let thrownError: unknown;

    try {
      await prefetchLocalReady(
        {
          timeoutMs: 10,
          intervalMs: 4,
        },
        {
          prefetchLocal: () => {
            attempts += 1;
            // wasm-bindgen rejects with structured objects that are not Error instances.
            // eslint-disable-next-line prefer-promise-reject-errors
            return Promise.reject({ message: 'quorum is not ready' });
          },
          now: () => timestamps.shift() ?? 10,
          sleep: async (durationMs) => {
            sleepDurations.push(durationMs);
          },
        },
      );
    } catch (error) {
      thrownError = error;
    }

    expect(attempts).to.equal(1);
    expect(sleepDurations).to.deep.equal([1]);
    expect(thrownError).to.be.instanceOf(Error);
    expect((thrownError as Error).message).to.equal(
      'prefetchLocalReady: local network not ready after 10ms '
      + '(1 attempts); last error: quorum is not ready',
    );
  });
});
