/**
 * `getContestFundToJoin` reads the contest a document enters from the network, so the unit
 * suite checks only what it does before any read: the document is taken apart first.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('getContestFundToJoin()', () => {
  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();
  });

  after(() => {
    client?.free();
  });

  it('should refuse a value that is not a Document before reading anything', async () => {
    await expect(client.getContestFundToJoin({} as never)).to.be.rejectedWith(/Document/);
  });
});
