import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Contested resources & voting', function describeContestedResources() {
  this.timeout(60000);

  let client;
  let builder;
  const { dpnsContractId, dpnsDomain } = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) {
      client.free();
    }
  });

  it('lists contested resources and vote state', async () => {
    const DPNS_CONTRACT = dpnsContractId;
    const PARENT = dpnsDomain.parent;
    const LABEL = dpnsDomain.label;

    await client.getContestedResources({
      dataContractId: DPNS_CONTRACT,
      documentTypeName: 'domain',
      indexName: 'parentNameAndLabel',
      orderAscending: true,
    });

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
