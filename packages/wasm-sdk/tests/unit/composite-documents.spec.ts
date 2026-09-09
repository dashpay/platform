import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.js';

describe('composite document query input', () => {
  let client: sdk.WasmSdk;

  before(async () => {
    await init();
  });

  beforeEach(async () => {
    client = await sdk.WasmSdkBuilder.testnet().build();
  });

  afterEach(() => {
    client.free();
  });

  const validQuery = (): sdk.CompositeDocumentsQuery => ({
    dataContractId: new Uint8Array(32).fill(42),
    documentType: 'post',
    limit: 10,
    subQueries: [{
      documentType: 'post',
      bind: { sourceProperty: 'quotedPostId', field: '$id' },
    }],
  });

  (['getCompositeDocuments', 'getCompositeDocumentsWithProofInfo'] as const).forEach((method) => {
    // Every case fails before a contract fetch. Running both real WASM exports
    // pins JS deserialization and the proof-info path without network calls.
    const cases: [string, () => object][] = [
      ['empty sub-queries', () => ({ ...validQuery(), subQueries: [] })],
      ['too many sub-queries', () => ({
        ...validQuery(), subQueries: Array(11).fill(validQuery().subQueries[0]),
      })],
      ['zero page limit', () => ({ ...validQuery(), limit: 0 })],
      ['oversized page limit', () => ({ ...validQuery(), limit: 101 })],
      ['fractional page limit', () => ({ ...validQuery(), limit: 1.5 })],
      ['missing page limit', () => ({ ...validQuery(), limit: undefined })],
      ['unsupported cursor', () => ({ ...validQuery(), startAfter: new Uint8Array(32) })],
      ['unknown sub-query fields', () => ({
        ...validQuery(), subQueries: [{ ...validQuery().subQueries[0], offset: 1 }],
      })],
      ['limit on a counts sub-query', () => ({
        ...validQuery(),
        subQueries: [{
          documentType: 'like',
          kind: 'counts',
          limit: 1,
          bind: { sourceProperty: '$id', field: 'postId' },
        }],
      })],
      ['self binding', () => ({
        ...validQuery(),
        subQueries: [{
          ...validQuery().subQueries[0],
          bind: { source: 0, sourceProperty: '$id', field: '$id' },
        }],
      })],
      ['fractional binding', () => ({
        ...validQuery(),
        subQueries: [{
          ...validQuery().subQueries[0],
          bind: { source: 0.5, sourceProperty: '$id', field: '$id' },
        }],
      })],
      ['binding to counts', () => ({
        ...validQuery(),
        subQueries: [
          { documentType: 'like', kind: 'counts', bind: { sourceProperty: '$id', field: 'postId' } },
          { documentType: 'post', bind: { source: 0, sourceProperty: '$id', field: '$id' } },
        ],
      })],
    ];

    cases.forEach(([name, query]) => {
      it(`should reject ${name} through ${method} before fetching`, async () => {
        let error: unknown;
        try {
          await client[method](query() as sdk.CompositeDocumentsQuery);
        } catch (caught) {
          error = caught;
        }
        expect(error).to.have.property('kind', sdk.WasmSdkErrorKind.InvalidArgument);
      });
    });
  });
});
