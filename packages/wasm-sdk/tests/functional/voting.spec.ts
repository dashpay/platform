import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Voting', function describeVoting() {
  this.timeout(60000);

  let client: sdk.WasmSdk;
  let builder: sdk.WasmSdkBuilder;
  const { dpnsContractId, dpnsDomain } = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    const context = await sdk.WasmTrustedContext.prefetchLocal();
    builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();
  });

  after(() => {
    if (client) {
      client.free();
    }
  });

  describe('getContestedResources()', () => {
    it('should list contested resources', async () => {
      const DPNS_CONTRACT = dpnsContractId;

      await client.getContestedResources({
        dataContractId: DPNS_CONTRACT,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        orderAscending: true,
      });
    });
  });

  describe('getContestedResourceVoteState()', () => {
    it('should get contested resource vote state', async () => {
      const DPNS_CONTRACT = dpnsContractId;
      const PARENT = dpnsDomain.parent;
      const LABEL = dpnsDomain.label;

      await client.getContestedResourceVoteState({
        dataContractId: DPNS_CONTRACT,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: [PARENT, LABEL],
        resultType: 'documents',
        limit: 50,
        includeLockedAndAbstaining: true,
      });
    });
  });
});
