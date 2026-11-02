/**
 * `masternodeVote` takes the masternode's ProTxHash as an `IdentifierLike` and reads it before
 * the other options, so a call whose only fault is its missing vote poll fails on the vote poll:
 * its ProTxHash was accepted, and nothing reached the network.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const proTxHashHex = 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

async function rejectionOf(promise: Promise<unknown>): Promise<Error> {
  try {
    await promise;
  } catch (e) {
    return e as Error;
  }
  throw new Error('expected the vote to be refused');
}

describe('masternodeVote()', () => {
  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();
  });

  const accepted: Array<[string, () => unknown]> = [
    ['an Identifier', () => new sdk.Identifier(proTxHashHex)],
    ['its 32 bytes', () => new sdk.Identifier(proTxHashHex).toBytes()],
    ['the hex Core shows', () => proTxHashHex],
    ['base58', () => new sdk.Identifier(proTxHashHex).toBase58()],
  ];

  accepted.forEach(([form, proTxHash]) => {
    it(`should accept the ProTxHash as ${form}`, async () => {
      const error = await rejectionOf(client.masternodeVote({
        masternodeProTxHash: proTxHash(),
      } as never));
      expect(error.message).to.match(/'votePoll' is required/);
    });
  });

  it('should refuse a ProTxHash that is not an identifier before the vote poll', async () => {
    const error = await rejectionOf(client.masternodeVote({
      masternodeProTxHash: 42,
    } as never));
    expect(error.message).to.not.match(/votePoll/);
  });
});
