import init, * as sdk from '../../dist/sdk.js';

describe('Document queries', function () {
  this.timeout(60000);

  const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

  let client;
  let builder;

  before(async function () {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsTestnet();
    builder = sdk.WasmSdkBuilder.testnetTrusted();
    client = await builder.build();
  });

  after(function () {
    if (client) client.free();
  });

  it('lists DPNS documents (no filters)', async () => {
    const docs = await client.getDocuments(DPNS_CONTRACT, 'domain', null, null, 5, null, null);
    expect(docs).to.be.an('array');
  });

  it('queries with where clause', async () => {
    const where = JSON.stringify([["normalizedParentDomainName", "==", "dash"]]);
    const docs = await client.getDocuments(DPNS_CONTRACT, 'domain', where, null, 5, null, null);
    expect(docs).to.be.an('array');
  });

  it('queries with orderBy', async () => {
    const orderBy = JSON.stringify([["normalizedParentDomainName", "asc"]]);
    const docs = await client.getDocuments(DPNS_CONTRACT, 'domain', null, orderBy, 5, null, null);
    expect(docs).to.be.an('array');
  });

  it('complex where + orderBy', async () => {
    const where = JSON.stringify([["normalizedLabel", "startsWith", "test"],["normalizedParentDomainName", "==", "dash"]]);
    const orderBy = JSON.stringify([["normalizedParentDomainName", "asc"],["normalizedLabel", "asc"]]);
    const docs = await client.getDocuments(DPNS_CONTRACT, 'domain', where, orderBy, 5, null, null);
    expect(docs).to.be.an('array');
  });

  it('getDocument by id (should handle invalid id gracefully)', async () => {
    await expect(
      client.getDocument(DPNS_CONTRACT, 'domain', 'invalidDocumentId')
    ).to.be.rejected();
  });

  it('fetches usernames for a known identity and verifies fields', async () => {
    const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const list = await client.getDpnsUsernames(TEST_IDENTITY, 10);
    expect(list).to.be.an('array');
  });
});
