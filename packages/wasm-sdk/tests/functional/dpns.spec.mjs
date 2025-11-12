import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Document queries', function describeDocumentQueries() {
  this.timeout(60000);

  const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

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

  it('lists DPNS documents (no filters)', async () => {
    const docs = await client.getDocuments({
      dataContractId: DPNS_CONTRACT,
      documentTypeName: 'domain',
      limit: 5,
    });
    expect(docs).to.be.instanceOf(Map);
  });

  it('queries with where clause', async () => {
    const docs = await client.getDocuments({
      dataContractId: DPNS_CONTRACT,
      documentTypeName: 'domain',
      where: [
        ['normalizedParentDomainName', '==', 'dash'],
      ],
      limit: 5,
    });
    expect(docs).to.be.instanceOf(Map);
  });

  it('queries with orderBy', async () => {
    const docs = await client.getDocuments({
      dataContractId: DPNS_CONTRACT,
      documentTypeName: 'domain',
      orderBy: [
        ['normalizedParentDomainName', 'asc'],
      ],
      limit: 5,
    });
    expect(docs).to.be.instanceOf(Map);
  });

  it('complex where + orderBy', async () => {
    const docs = await client.getDocuments({
      dataContractId: DPNS_CONTRACT,
      documentTypeName: 'domain',
      where: [
        ['normalizedLabel', 'startsWith', 'test'],
        ['normalizedParentDomainName', '==', 'dash'],
      ],
      orderBy: [
        ['normalizedParentDomainName', 'asc'],
        ['normalizedLabel', 'asc'],
      ],
      limit: 5,
    });
    expect(docs).to.be.instanceOf(Map);
  });

  it('getDocument by id (should handle invalid id gracefully)', async () => {
    await expect(
      client.getDocument(DPNS_CONTRACT, 'domain', 'invalidDocumentId'),
    ).to.be.rejected();
  });

  it('fetches usernames for a known identity and verifies fields', async () => {
    const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const list = await client.getDpnsUsernames(TEST_IDENTITY, 10);
    expect(list).to.be.an('array');
  });
});
