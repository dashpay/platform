import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import proof from '../../../rs-sdk-trusted-context-provider/tests/data/proof-testnet.json' with { type: 'json' };

// RFC 6761 reserved test domain: `fetch` is replaced, so nothing is dialled.
const RELAY = 'https://relay.test';
// The quorum the testnet fixture proof opens: its type, its hash in RPC display order and
// its key (the Rust test `should_name_the_quorum_the_fixture_proof_opens` reads them from
// the proof).
const QUORUM_TYPE = 6;
const QUORUM_HASH = '0000001b381f694e98ab6efcf1f380aba750bd6b4f70f48f9e8818cff2a64af6';
const QUORUM_KEY = 'b813d560b9a73db7b01cfe80ee7ab20a4af7ccda4b94a0cd8ceda8f5cb0fa604760430ef8c644e0f823e089aa7839f93';

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function errorMessage(error: unknown): string {
  return (error as { message?: string })?.message ?? String(error);
}

describe('WasmVerifiedContext', () => {
  before(async () => {
    await init();
  });

  describe('with the embedded checkpoint', () => {
    it('should start from the embedded checkpoint before any proof', () => {
      for (const [factory, network] of [
        [() => sdk.WasmVerifiedContext.testnet(), 1],
        [() => sdk.WasmVerifiedContext.mainnet(), 0],
      ] as const) {
        const context = factory();
        const checkpoint = context.checkpoint();
        expect(checkpoint.network).to.equal(network);
        expect(checkpoint.height).to.be.a('number');
        expect(checkpoint.block_hash).to.match(/^[0-9a-f]{64}$/);
        expect(context.latestVerifiedState()).to.deep.equal(checkpoint);
        context.free();
      }
    });

    it('should ask the network\'s public quorum service unless relays are given', () => {
      const context = sdk.WasmVerifiedContext.testnet();
      expect(context.relays()).to.deep.equal(['https://quorums.testnet.networks.dash.org/']);
      context.free();

      const custom = sdk.WasmVerifiedContext.testnet([RELAY]);
      expect(custom.relays()).to.deep.equal([`${RELAY}/`]);
      custom.free();
    });

    it('should accept a saved state as the checkpoint', () => {
      const embedded = sdk.WasmVerifiedContext.testnet();
      // What an app would store and read back.
      const saved = JSON.parse(JSON.stringify(embedded.latestVerifiedState()));
      const restored = sdk.WasmVerifiedContext.testnetWithCheckpoint(saved);
      expect(restored.checkpoint()).to.deep.equal(embedded.checkpoint());
      embedded.free();
      restored.free();
    });
  });

  describe('invalid input', () => {
    it('should reject a checkpoint that is not an object', () => {
      expect(() => sdk.WasmVerifiedContext.testnetWithCheckpoint(
        JSON.stringify(proof.checkpoint) as unknown as sdk.QuorumProofState,
      )).to.throw(/QuorumProofState object/);
    });

    it('should reject a malformed checkpoint', () => {
      expect(() => sdk.WasmVerifiedContext.testnetWithCheckpoint(
        { ...proof.checkpoint, block_hash: 'zz' },
      )).to.throw(/Invalid quorum proof state/);
      expect(() => sdk.WasmVerifiedContext.testnetWithCheckpoint(
        { ...proof.checkpoint, extra: 1 } as unknown as sdk.QuorumProofState,
      )).to.throw(/Invalid quorum proof state/);
    });

    it('should reject a checkpoint of the other network', () => {
      expect(() => sdk.WasmVerifiedContext.mainnetWithCheckpoint(proof.checkpoint, [RELAY]))
        .to.throw(/checkpoint/);
    });

    it('should reject relays the provider may not ask', () => {
      for (const relays of [
        [],
        ['http://relay.test'],
        ['not a url'],
        [RELAY, `${RELAY}/`],
        Array.from({ length: 9 }, (_, i) => `https://relay${i}.test`),
      ]) {
        expect(() => sdk.WasmVerifiedContext.testnetWithCheckpoint(proof.checkpoint, relays), String(relays))
          .to.throw(/proof source/);
      }
    });

    it('should reject a quorum hash that is not 32 bytes of hex', async () => {
      const context = sdk.WasmVerifiedContext.testnet([RELAY]);
      let thrown: unknown;
      try {
        await context.fetchQuorumPublicKey(QUORUM_TYPE, 'abcd', 1);
      } catch (error) {
        thrown = error;
      }
      expect(errorMessage(thrown)).to.match(/quorumHash must be 32 bytes of hex/);
      context.free();
    });
  });

  describe('WasmSdkBuilder.withVerifiedContext()', () => {
    it('should build an SDK with the verified context attached', () => {
      const context = sdk.WasmVerifiedContext.testnet();
      const built = sdk.WasmSdkBuilder.testnet().withVerifiedContext(context).build();
      expect(built).to.be.ok();
      built.free();
      // The builder keeps a clone: the caller's context stays usable.
      expect(context.latestVerifiedState()).to.deep.equal(context.checkpoint());
      context.free();
    });

    describe('with a trusted context', () => {
      const originalFetch = globalThis.fetch;
      let trusted: sdk.WasmTrustedContext;

      before(async () => {
        // An empty quorum service, enough to prefetch a trusted context.
        globalThis.fetch = async (input: RequestInfo | URL, options?: RequestInit) => {
          const request = new Request(input, options);
          const data = new URL(request.url).pathname === '/previous' ? { height: 1, quorums: [] } : [];
          const response = new Response(JSON.stringify({ success: true, data }), {
            headers: { 'content-type': 'application/json' },
          });
          // Synthetic Fetch responses must carry the request URL, as real responses do.
          Object.defineProperty(response, 'url', { value: request.url });
          return response;
        };
        try {
          trusted = await sdk.WasmTrustedContext.prefetchTestnetWithUrl('https://quorums.test', false);
        } finally {
          globalThis.fetch = originalFetch;
        }
      });

      it('should refuse a trusted context after a verified one', () => {
        const context = sdk.WasmVerifiedContext.testnet();
        const builder = sdk.WasmSdkBuilder.testnet().withVerifiedContext(context);
        expect(() => builder.withTrustedContext(trusted)).to.throw(/already attached/);
        context.free();
      });

      it('should refuse a verified context after a trusted one', () => {
        const context = sdk.WasmVerifiedContext.testnet();
        const builder = sdk.WasmSdkBuilder.testnet().withTrustedContext(trusted);
        expect(() => builder.withVerifiedContext(context)).to.throw(/already attached/);
        context.free();
      });
    });
  });

  // Goes through the browser's `fetch` end to end: the provider POSTs the proof
  // request, reads the streamed body, verifies the proof and opens the key.
  describe('fetchQuorumPublicKey() through fetch', () => {
    const originalFetch = globalThis.fetch;
    let requests: { url: string; method: string; body: Record<string, unknown> }[];

    beforeEach(() => {
      requests = [];
      globalThis.fetch = async (input: RequestInfo | URL, options?: RequestInit) => {
        const request = new Request(input, options);
        requests.push({ url: request.url, method: request.method, body: await request.json() });
        return new Response(fromHex(proof.bootstrap_hex), { status: 200 });
      };
    });

    afterEach(() => {
      globalThis.fetch = originalFetch;
    });

    it('should prove the key a relay\'s proof opens and advance the latest verified state', async () => {
      const context = sdk.WasmVerifiedContext.testnetWithCheckpoint(proof.checkpoint, [RELAY]);
      expect(context.latestVerifiedState()).to.deep.equal(proof.checkpoint);

      const started = performance.now();
      const key = await context.fetchQuorumPublicKey(QUORUM_TYPE, QUORUM_HASH, proof.target.height);
      // The proof is verified inside this call, on the event loop (#5706).
      // eslint-disable-next-line no-console
      console.log(`fetchQuorumPublicKey with a 4-link testnet proof: ${(performance.now() - started).toFixed(1)} ms`);

      expect(toHex(key)).to.equal(QUORUM_KEY);
      expect(context.latestVerifiedState()).to.deep.equal(proof.target);
      expect(requests).to.have.length(1);
      expect(requests[0].url).to.equal(`${RELAY}/proofs`);
      expect(requests[0].method).to.equal('POST');
      expect(requests[0].body).to.include({
        height: proof.target.height,
        quorumHash: QUORUM_HASH,
        llmqType: QUORUM_TYPE,
      });

      // A key already proven for the height is served without another request.
      const again = await context.fetchQuorumPublicKey(QUORUM_TYPE, QUORUM_HASH, proof.target.height);
      expect(toHex(again)).to.equal(QUORUM_KEY);
      expect(requests).to.have.length(1);
      context.free();
    });

    it('should report a proof that does not verify', async () => {
      globalThis.fetch = async () => new Response(new Uint8Array([1, 2, 3]), { status: 200 });
      const context = sdk.WasmVerifiedContext.testnetWithCheckpoint(proof.checkpoint, [RELAY]);
      let thrown: unknown;
      try {
        await context.fetchQuorumPublicKey(QUORUM_TYPE, QUORUM_HASH, proof.target.height);
      } catch (error) {
        thrown = error;
      }
      expect(errorMessage(thrown)).to.match(/Failed to prove the quorum public key/);
      expect(context.latestVerifiedState()).to.deep.equal(proof.checkpoint);
      context.free();
    }).timeout(120000);
  });
});
