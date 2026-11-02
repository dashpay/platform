/**
 * `getContractDocumentRemovals`, `getContractTeamActions` and `getContractTeamActionSigners`
 * take `IdentifierLike` contract ids, document ids, action ids and cursors. Each field is
 * parsed on its own before anything is fetched, and the fault each case carries (a cursor beside
 * `documentIds`, a status that is neither `active` nor `closed`) is refused after every other
 * field parsed, so a query whose only fault is that one fails on it: its identifiers and its
 * limit were accepted, and nothing reached the network.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const contractId = 'EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88';
const documentId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const actionId = 'cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN';
const pairRefused = /`startAfter` and `limit` page through every record and are not valid beside `documentIds`/;
const statusRefused = /`status` must be 'active' or 'closed', got 'pending'/;

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
    it(`should accept ${form} in a document removals query`, async () => {
      const error = await rejectionOf(client.getContractDocumentRemovals({
        contractId: as(contractId),
        documentTypeName: 'post',
        documentIds: [as(documentId)],
        startAfter: as(documentId),
      } as never));
      expect(error.message).to.match(pairRefused);
    });

    it(`should accept ${form} in a team actions query`, async () => {
      const error = await rejectionOf(client.getContractTeamActions({
        contractId: as(contractId),
        status: 'pending',
        startAtActionId: as(actionId),
        startAtActionIdIncluded: true,
      } as never));
      expect(error.message).to.match(statusRefused);
    });

    it(`should accept ${form} in a team action signers query`, async () => {
      const error = await rejectionOf(client.getContractTeamActionSigners({
        contractId: as(contractId),
        status: 'pending',
        actionId: as(actionId),
      } as never));
      expect(error.message).to.match(statusRefused);
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

  it('should accept a BigInt limit in a team actions query', async () => {
    const error = await rejectionOf(client.getContractTeamActions({
      contractId,
      status: 'pending',
      limit: 50n,
    } as never));
    expect(error.message).to.match(statusRefused);
  });

  it('should refuse a team actions limit outside 1 to the page cap before any request', async () => {
    for (const limit of [0, 101n]) {
      const error = await rejectionOf(client.getContractTeamActions({
        contractId,
        status: 'active',
        limit,
      } as never));
      expect(error.message).to.match(/`limit` must be between 1 and 100/);
    }
  });

  it('should refuse a team actions inclusion flag without the action it starts at', async () => {
    const error = await rejectionOf(client.getContractTeamActions({
      contractId,
      status: 'closed',
      startAtActionIdIncluded: true,
    } as never));
    expect(error.message).to.match(/`startAtActionIdIncluded` is only valid beside `startAtActionId`/);
  });

  it('should refuse a document id that is not an identifier, naming the query', async () => {
    const error = await rejectionOf(client.getContractDocumentRemovals({
      contractId,
      documentTypeName: 'post',
      documentIds: [42],
    } as never));
    expect(error.message).to.match(/Invalid contract document removals query/);
  });

  it('should refuse an action id that is not an identifier, naming the query', async () => {
    const error = await rejectionOf(client.getContractTeamActionSigners({
      contractId,
      status: 'active',
      actionId: 42,
    } as never));
    expect(error.message).to.match(/Invalid contract team action signers query/);
  });
});
