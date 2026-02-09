import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Groups', function describeGroups() {
  this.timeout(60000);

  let client: sdk.WasmSdk;
  let builder: sdk.WasmSdkBuilder;

  before(async () => {
    await init();
    const context = await sdk.WasmTrustedContext.prefetchLocal();
    builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('getIdentityGroups()', () => {
    it('should fetch identity groups', async () => {
      const { dpnsContractId: DPNS_CONTRACT, identityId: IDENTITY } = (
        wasmFunctionalTestRequirements()
      );
      // These calls may fail in offline runs; permit network errors
      await client.getIdentityGroups({
        identityId: IDENTITY,
        memberDataContracts: [DPNS_CONTRACT],
      });
    });
  });

  describe('getGroupMembers()', () => {
    it('should fetch group members', async () => {
      const { dpnsContractId: DPNS_CONTRACT } = wasmFunctionalTestRequirements();
      await client.getGroupMembers({
        dataContractId: DPNS_CONTRACT,
        groupContractPosition: 0,
        limit: 10,
      });
    });
  });

  describe('getGroupsDataContracts()', () => {
    it('should fetch groups data contracts', async () => {
      const { dpnsContractId: DPNS_CONTRACT } = wasmFunctionalTestRequirements();
      await client.getGroupsDataContracts([DPNS_CONTRACT]);
    });
  });
});
