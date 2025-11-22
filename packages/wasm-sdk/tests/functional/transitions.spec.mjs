import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

// TODO: Implement tests for all state transitions factories

describe('Basic state transitions', function describeBasicStateTransitions() {
  this.timeout(60000);

  let client;
  let builder;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsTestnet();
    builder = sdk.WasmSdkBuilder.testnetTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  it.skip('tokenTransfer rejects invalid parameters', async () => {
    const { tokenContracts, identityId } = wasmFunctionalTestRequirements();
    await client.tokenTransfer(tokenContracts[0].contractId, 0, '1000', identityId, identityId, 'Kx...', null);
  });
});
