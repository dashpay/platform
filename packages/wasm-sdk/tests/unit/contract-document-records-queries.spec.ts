/**
 * `getContractSettledDeletions` and `getContractDocumentRemovals` take `IdentifierLike`
 * contract ids, document ids and cursors. Each field is parsed on its own before anything is
 * fetched, and a cursor beside `documentIds` is refused after every field parsed, so a query
 * whose only fault is that pair fails on it: its identifiers were accepted, and nothing
 * reached the network.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const contractId = 'EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88';
const documentId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const pairRefused = /`startAfter` and `limit` page through every record and are not valid beside `documentIds`/;

async function rejectionOf(promise: Promise<unknown>): Promise<Error> {
  try {
    await promise;
  } catch (e) {
    return e as Error;
  }
  throw new Error('expected the query to be refused');
}

describe('contract document records queries', () => {
  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();
  });

  const forms: Array<[string, (id: string) => unknown]> = [
    ['Identifier instances', (id) => new sdk.Identifier(id)],
    ['base58 strings', (id) => id],
    ['Uint8Arrays', (id) => new sdk.Identifier(id).toBytes()],
  ];

  forms.forEach(([form, as]) => {
    it(`should accept ${form} in a settled deletions query`, async () => {
      const error = await rejectionOf(client.getContractSettledDeletions({
        contractId: as(contractId),
        documentTypeName: 'post',
        documentIds: [as(documentId)],
        startAfter: as(documentId),
      } as never));
      expect(error.message).to.match(pairRefused);
    });

    it(`should accept ${form} in a document removals query`, async () => {
      const error = await rejectionOf(client.getContractDocumentRemovals({
        contractId: as(contractId),
        documentTypeName: 'post',
        documentIds: [as(documentId)],
        startAfter: as(documentId),
      } as never));
      expect(error.message).to.match(pairRefused);
    });
  });

  it('should accept a BigInt limit in a document removals query', async () => {
    const error = await rejectionOf(client.getContractDocumentRemovals({
      contractId,
      documentTypeName: 'post',
      documentIds: [documentId],
      limit: 50n,
    } as never));
    expect(error.message).to.match(pairRefused);
  });

  it('should refuse a document id that is not an identifier, naming the query', async () => {
    const error = await rejectionOf(client.getContractSettledDeletions({
      contractId,
      documentTypeName: 'post',
      documentIds: [42],
    } as never));
    expect(error.message).to.match(/Invalid contract settled deletions query/);
  });
});
