import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Protocol versions', function describeProtocolVersions() {
  this.timeout(60000);

  let client;
  let evonodeProTxHash;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();

    // Get the proTxHash from the node status
    const status = await client.getStatus();
    evonodeProTxHash = status.node.proTxHash;
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('fetches protocol upgrade state', async () => {
    const state = await client.getProtocolVersionUpgradeState();
    expect(state).to.be.ok();
  });

  it('fetches protocol upgrade state with proof', async () => {
    const res = await client.getProtocolVersionUpgradeStateWithProofInfo();
    expect(res).to.be.ok();
    expect(res.data).to.be.ok();
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
  });

  it('lists protocol upgrade vote statuses', async () => {
    // Use evonodeProTxHash if available, otherwise start from beginning with empty string
    const startProTxHash = evonodeProTxHash || '';
    const res = await client.getProtocolVersionUpgradeVoteStatus(startProTxHash, 50);
    expect(res).to.be.instanceOf(Map);
  });

  it('lists protocol upgrade vote statuses with proof', async () => {
    // Use evonodeProTxHash if available, otherwise start from beginning with empty string
    const startProTxHash = evonodeProTxHash || '';
    const res = await client.getProtocolVersionUpgradeVoteStatusWithProofInfo(startProTxHash, 50);
    expect(res).to.be.ok();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
  });
});
