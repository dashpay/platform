/**
 * The page queries of `getModerationSubmittedCharters` and `getModerationJoinRequests` take
 * `IdentifierLike` ids and cursors. Each field is parsed on its own before anything is
 * fetched, and `limit` is parsed last, so a query whose only fault is its limit fails on the
 * limit: its identifiers were accepted, and nothing reached the network.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const contractId = 'EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88';

async function rejectionOf(promise: Promise<unknown>): Promise<Error> {
  try {
    await promise;
  } catch (e) {
    return e as Error;
  }
  throw new Error('expected the query to be refused');
}

describe('moderation charters page queries', () => {
  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();
  });

  it('should accept Identifier instances as the proposal query id and cursor', async () => {
    const error = await rejectionOf(client.getModerationSubmittedCharters({
      targetContractId: new sdk.Identifier(contractId),
      startAfter: new sdk.Identifier(contractId),
      limit: 1.5,
    }));
    expect(error.message).to.match(/'limit' must be an integer/);
  });

  it('should accept Identifier instances as the join request query id and cursor', async () => {
    const error = await rejectionOf(client.getModerationJoinRequests({
      submittedCharterId: new sdk.Identifier(contractId),
      startAfter: new sdk.Identifier(contractId),
      limit: 1.5,
    }));
    expect(error.message).to.match(/'limit' must be an integer/);
  });

  it('should accept base58 strings as the query id and cursor', async () => {
    const error = await rejectionOf(client.getModerationJoinRequests({
      submittedCharterId: contractId,
      startAfter: contractId,
      limit: 1.5,
    }));
    expect(error.message).to.match(/'limit' must be an integer/);
  });

  it('should refuse a query id that is not an identifier before the limit', async () => {
    const error = await rejectionOf(client.getModerationSubmittedCharters({
      targetContractId: 42 as never,
      limit: 1.5,
    }));
    expect(error.message).to.not.match(/limit/);
  });
});
