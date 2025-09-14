import init, * as sdk from '../../dist/sdk.js';

const TOKEN_CONTRACT = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';

// TODO: Implement tests for all state transitions factories

describe('Basic state transitions', function () {
  this.timeout(60000);

  let client;
  let builder;

  before(async function () {
    await init();
    await sdk.prefetch_trusted_quorums_testnet();
    builder = sdk.WasmSdkBuilder.new_testnet_trusted();
    client = await builder.build();
  });

  after(function () {
    if (client) client.free();
  });

  it.skip('tokenTransfer rejects invalid parameters', async () => {
    await client.tokenTransfer(TOKEN_CONTRACT, 0, '1000', TEST_IDENTITY, TEST_IDENTITY, 'Kx...', null)
  });
});
