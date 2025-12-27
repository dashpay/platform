import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Protocol versions', function describeProtocolVersions() {
  this.timeout(60000);

  let client;
  let builder;
  const { evonodeProTxHash } = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('fetches protocol upgrade state', async () => {
    const state = await client.getProtocolVersionUpgradeState();
    expect(state).to.be.ok();
  });

  it('lists protocol upgrade vote statuses', async function listsVoteStatuses() {
    if (!evonodeProTxHash) {
      this.skip();
    }
    const START_PROTX = evonodeProTxHash;
    const res = await client.getProtocolVersionUpgradeVoteStatus(START_PROTX, 50);
    expect(res).to.be.instanceOf(Map);
  });
});
