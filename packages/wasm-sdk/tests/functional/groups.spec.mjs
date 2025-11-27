import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Group queries', function describeGroupQueries() {
  this.timeout(60000);

  let client;
  let builder;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('fetches identity groups and group members', async () => {
    const { dpnsContractId: DPNS_CONTRACT, identityId: IDENTITY } = wasmFunctionalTestRequirements();
    // These calls may fail in offline runs; permit network errors
    await client.getIdentityGroups({
      identityId: IDENTITY,
      memberDataContracts: [DPNS_CONTRACT],
    });
    await client.getGroupMembers({
      dataContractId: DPNS_CONTRACT,
      groupContractPosition: 0,
      limit: 10,
    });
  });

  it('fetches groups data contracts', async () => {
    const { dpnsContractId: DPNS_CONTRACT } = wasmFunctionalTestRequirements();
    await client.getGroupsDataContracts([DPNS_CONTRACT]);
  });
});
