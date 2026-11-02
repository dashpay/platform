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

/** The message a promise rejects with; wasm-bindgen rejects with non-Error objects. */
async function rejection(promise: Promise<unknown>): Promise<string> {
  try {
    await promise;
  } catch (error) {
    return (error as { message?: string })?.message ?? String(error);
  }
  throw new Error('expected a rejection');
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
        expect(context.continuation()).to.deep.equal({ anchor: checkpoint, states: [] });
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

    it('should accept a checkpoint and a continuation in their JSON forms', () => {
      const embedded = sdk.WasmVerifiedContext.testnet();
      // What an app would store and read back.
      const checkpoint = JSON.parse(JSON.stringify(embedded.checkpoint()));
      const fromCheckpoint = sdk.WasmVerifiedContext.testnetWithCheckpoint(checkpoint);
      expect(fromCheckpoint.checkpoint()).to.deep.equal(embedded.checkpoint());
      const continuation = JSON.parse(JSON.stringify(embedded.continuation()));
      const resumed = sdk.WasmVerifiedContext.testnetFromContinuation(continuation);
      expect(resumed.continuation()).to.deep.equal(continuation);
      embedded.free();
      fromCheckpoint.free();
      resumed.free();
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
      )).to.throw(/Invalid checkpoint/);
      expect(() => sdk.WasmVerifiedContext.testnetWithCheckpoint(
        { ...proof.checkpoint, extra: 1 } as unknown as sdk.QuorumProofState,
      )).to.throw(/Invalid checkpoint/);
    });

    it('should reject a continuation it cannot resume from', () => {
      expect(() => sdk.WasmVerifiedContext.testnetFromContinuation(
        JSON.stringify({ anchor: proof.checkpoint, states: [] }) as unknown as sdk.QuorumProofContinuation,
      )).to.throw(/QuorumProofContinuation object/);
      expect(() => sdk.WasmVerifiedContext.testnetFromContinuation(
        { anchor: proof.checkpoint } as unknown as sdk.QuorumProofContinuation,
      )).to.throw(/Invalid continuation/);
      // A state must lie above the anchor.
      expect(() => sdk.WasmVerifiedContext.testnetFromContinuation(
        { anchor: proof.target, states: [proof.checkpoint] },
      )).to.throw(/not above the anchor/);
    });

    it('should reject a checkpoint of the other network', () => {
      expect(() => sdk.WasmVerifiedContext.mainnetWithCheckpoint(proof.checkpoint, [RELAY]))
        .to.throw(/checkpoint/);
    });

    // The provider's own tests cover which relays it refuses; this checks the refusal throws.
    it('should reject a relay the provider may not ask', () => {
      expect(() => sdk.WasmVerifiedContext.testnetWithCheckpoint(proof.checkpoint, ['http://relay.test']))
        .to.throw(/proof source/);
    });

    it('should reject a quorum hash that is not 32 bytes of hex', async () => {
      const context = sdk.WasmVerifiedContext.testnet([RELAY]);
      expect(await rejection(context.fetchQuorumPublicKey(QUORUM_TYPE, 'abcd', 1)))
        .to.match(/quorumHash must be 32 bytes of hex/);
      context.free();
    });
  });

  describe('WasmSdkBuilder.withVerifiedContext()', () => {
    it('should refuse to build with a context of another network', () => {
      const context = sdk.WasmVerifiedContext.mainnet();
      const builder = sdk.WasmSdkBuilder.testnet().withVerifiedContext(context);
      expect(() => builder.build()).to.throw(/Mainnet/);
      context.free();
    });

    it('should refuse to build a verified SDK with proofs turned off', () => {
      const context = sdk.WasmVerifiedContext.testnet();
      const builder = sdk.WasmSdkBuilder.testnet().withVerifiedContext(context).withProofs(false);
      expect(() => builder.build()).to.throw(/requires proofs/);
      context.free();
    });

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

      // As in the Rust SDK's builder, the context attached last replaces the
      // one before it, of either kind.
      it('should build with either kind of context attached after the other', () => {
        const context = sdk.WasmVerifiedContext.testnet();
        const verifiedLast = sdk.WasmSdkBuilder.testnet()
          .withTrustedContext(trusted)
          .withVerifiedContext(context)
          .build();
        const trustedLast = sdk.WasmSdkBuilder.testnet()
          .withVerifiedContext(context)
          .withTrustedContext(trusted)
          .build();
        expect(verifiedLast).to.be.an.instanceof(sdk.WasmSdk);
        expect(trustedLast).to.be.an.instanceof(sdk.WasmSdk);
        verifiedLast.free();
        trustedLast.free();
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

    it('should prove the same key at the same height again after resuming from the continuation', async () => {
      const first = sdk.WasmVerifiedContext.testnetWithCheckpoint(proof.checkpoint, [RELAY]);
      await first.fetchQuorumPublicKey(QUORUM_TYPE, QUORUM_HASH, proof.target.height);
      // The key was served at the target; the checkpoint is the only state below it.
      expect(first.continuation()).to.deep.equal({ anchor: proof.checkpoint, states: [proof.target] });
      const saved = JSON.parse(JSON.stringify(first.continuation()));
      first.free();

      const next = sdk.WasmVerifiedContext.testnetFromContinuation(saved, [RELAY]);
      const key = await next.fetchQuorumPublicKey(QUORUM_TYPE, QUORUM_HASH, proof.target.height);

      expect(toHex(key)).to.equal(QUORUM_KEY);
      expect(requests).to.have.length(2);
      expect(requests[1].body).to.include({
        checkpoint: proof.checkpoint.block_hash,
        height: proof.target.height,
      });
      next.free();
    });

    it('should not prove a key at the height of a lone saved state', async () => {
      // What restarting from `latestVerifiedState()` alone would do.
      const stranded = sdk.WasmVerifiedContext.testnetWithCheckpoint(proof.target, [RELAY]);
      expect(await rejection(stranded.fetchQuorumPublicKey(QUORUM_TYPE, QUORUM_HASH, proof.target.height)))
        .to.match(/no verified state is below/);
      expect(requests).to.have.length(0);
      stranded.free();
    });

    it('should report a proof that does not verify', async () => {
      globalThis.fetch = async () => new Response(new Uint8Array([1, 2, 3]), { status: 200 });
      const context = sdk.WasmVerifiedContext.testnetWithCheckpoint(proof.checkpoint, [RELAY]);
      expect(await rejection(context.fetchQuorumPublicKey(QUORUM_TYPE, QUORUM_HASH, proof.target.height)))
        .to.match(/Failed to prove the quorum public key/);
      expect(context.latestVerifiedState()).to.deep.equal(proof.checkpoint);
      context.free();
    });
  });
});
