import init, * as sdk from '../../dist/sdk.js';

describe('User posts by identity', function () {
  this.timeout(60000);

  const IDENTITY_ID = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
  const CONTRACT_ID = '9nzpvjVSStUrhkEs3eNHw2JYpcNoLh1MjmqW45QiyjSa';

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
    if (builder) builder.free();
  });

  it('queries recent posts for identity owner', async () => {
    const whereClause = JSON.stringify([[ '$ownerId', '==', IDENTITY_ID ]]);
    const orderBy = JSON.stringify([[ '$createdAt', 'desc' ]]);

    const docs = await sdk.get_documents(
      client,
      CONTRACT_ID,
      'post',
      whereClause,
      orderBy,
      10,
      null,
      null,
    );

    expect(docs).to.be.an('array');
  });
});
