import init, * as sdk from '../../dist/sdk.compressed.js';

const TOKEN_CONTRACT = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';

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
    await client.tokenTransfer(TOKEN_CONTRACT, 0, '1000', TEST_IDENTITY, TEST_IDENTITY, 'Kx...', null);
  });
});
