import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import vectors from '../fixtures/state-transition-waits/signed-responses.json' with { type: 'json' };

const DAPI_URL = 'https://dapi.test:1443';
const QUORUM_URL = 'https://quorums.test:22444';
// The transport appends the RPC path to the address's trailing slash.
const RPC_PATH = '//org.dash.platform.dapi.v0.Platform';

function frame(flag: number, bytes: Uint8Array): Uint8Array {
  const result = new Uint8Array(5 + bytes.length);
  result[0] = flag;
  new DataView(result.buffer).setUint32(1, bytes.length);
  result.set(bytes, 5);
  return result;
}

function grpcResponse(bytes: number[]): Response {
  const data = frame(0, Uint8Array.from(bytes));
  const trailer = frame(0x80, new TextEncoder().encode('grpc-status:0\r\n'));
  const body = new Uint8Array(data.length + trailer.length);
  body.set(data);
  body.set(trailer, data.length);
  return new Response(body, { headers: { 'content-type': 'application/grpc-web+proto' } });
}

function hex(bytes: number[]): string {
  return bytes.map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

async function checkRequest(request: Request, expected: number[]): Promise<void> {
  expect(request.method).to.equal('POST');
  const bytes = new Uint8Array(await request.arrayBuffer());
  expect(bytes[0]).to.equal(0);
  expect(new DataView(bytes.buffer).getUint32(1)).to.equal(bytes.length - 5);
  expect(Array.from(bytes.subarray(5))).to.deep.equal(expected);
}

async function withFixture(
  run: (
    client: sdk.WasmSdk,
    transition: sdk.StateTransition,
    counts: { broadcasts: number; waits: number },
  ) => Promise<void>,
): Promise<void> {
  const originalFetch = globalThis.fetch;
  const OriginalDate = globalThis.Date;
  const counts = { broadcasts: 0, waits: 0 };
  let client: sdk.WasmSdk | undefined;
  let transition: sdk.StateTransition | undefined;

  try {
    // The fixture's signed timestamp is fixed; chrono constructs Date rather than calling now.
    globalThis.Date = new Proxy(OriginalDate, {
      construct(target, args) {
        return Reflect.construct(target, args.length ? args : [vectors.inputs.time_ms]);
      },
      apply() {
        return new OriginalDate(vectors.inputs.time_ms).toString();
      },
      get(target, property, receiver) {
        return property === 'now' ? () => vectors.inputs.time_ms : Reflect.get(target, property, receiver);
      },
    });
    globalThis.fetch = async (input, options) => {
      const request = input instanceof Request ? input : new Request(input, options);
      const respond = (response: Response): Response => {
        // Synthetic Fetch responses must carry the request URL, as real responses do.
        Object.defineProperty(response, 'url', { value: request.url });
        return response;
      };
      if (request.url === `${QUORUM_URL}/quorums`) {
        expect(request.method).to.equal('GET');
        return respond(new Response(JSON.stringify({
          success: true,
          data: [{
            quorum_hash: '46'.repeat(32),
            key: hex(vectors.quorum_public_key),
            height: 1,
            valid_members_count: 100,
          }],
        }), { headers: { 'content-type': 'application/json' } }));
      }
      if (request.url === `${QUORUM_URL}/previous`) {
        expect(request.method).to.equal('GET');
        return respond(new Response(
          JSON.stringify({ success: true, data: { height: 1, quorums: [] } }),
          { headers: { 'content-type': 'application/json' } },
        ));
      }
      if (request.url === `${DAPI_URL}${RPC_PATH}/broadcastStateTransition`) {
        await checkRequest(request, vectors.snapshot.broadcast_request_bytes);
        counts.broadcasts += 1;
        return respond(grpcResponse([]));
      }
      if (request.url === `${DAPI_URL}${RPC_PATH}/waitForStateTransitionResult`) {
        await checkRequest(request, vectors.snapshot.wait_request_bytes);
        counts.waits += 1;
        return respond(grpcResponse(vectors.snapshot.response_bytes));
      }
      throw new Error(`Unexpected fixture request: ${request.method} ${request.url}`);
    };

    const context = await sdk.WasmTrustedContext.prefetchLocalWithUrl(QUORUM_URL, false);
    client = await sdk.WasmSdkBuilder.withAddresses([DAPI_URL], 'local')
      .withVersion(vectors.protocol_version)
      .withTrustedContext(context)
      .withSettings(1000, 1000, 0, true)
      .build();
    transition = sdk.StateTransition.fromBytes(Uint8Array.from(vectors.snapshot.transition_bytes));
    await run(client, transition, counts);
  } finally {
    globalThis.fetch = originalFetch;
    globalThis.Date = OriginalDate;
    transition?.free();
    client?.free();
  }
}

describe('Verified state transition waits', () => {
  before(async () => {
    await init();
  });

  it('should authenticate and return an affected-state result through the public JS API', async () => {
    await withFixture(async (client, transition, counts) => {
      const result = await client.broadcastAndWaitForAffectedState(transition);
      try {
        expect(result).to.be.instanceOf(sdk.VerifiedContractModerationListStatuses);
        if (!(result instanceof sdk.VerifiedContractModerationListStatuses)) {
          throw new Error('Expected the authenticated moderation list wrapper');
        }
        expect(result.banned).to.equal(false);
        expect(result.lists).to.deep.equal(['banlist']);
        expect('height' in result).to.equal(false);
        expect(counts).to.deep.equal({ broadcasts: 1, waits: 1 });
      } finally {
        result.free();
      }
    });
  });

  it('should report execution not proved after accepted broadcast without rebroadcasting', async () => {
    await withFixture(async (client, transition, counts) => {
      let caught: unknown;
      try {
        await client.broadcastAndWait(transition);
      } catch (error) {
        caught = error;
      }
      expect(caught).to.be.instanceOf(sdk.WasmSdkError);
      if (!(caught instanceof sdk.WasmSdkError)) {
        throw new Error('Expected an actual rejected WASM error');
      }
      try {
        expect(caught.kind).to.equal(sdk.WasmSdkErrorKind.ExecutionNotProved);
        expect(caught.code).to.equal(-1);
        expect(caught.isRetriable).to.equal(false);
        expect(caught.message).to.include('waitForAffectedState');
        expect(counts).to.deep.equal({ broadcasts: 1, waits: 1 });
        expect(caught.message).not.to.include('Failed to broadcast');
        const result = await client.waitForAffectedState(transition);
        result.free();
        expect(counts).to.deep.equal({ broadcasts: 1, waits: 2 });
      } finally {
        caught.free();
      }
    });
  });
});
