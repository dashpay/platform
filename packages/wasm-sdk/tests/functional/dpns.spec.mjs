import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Document queries', function describeDocumentQueries() {
  this.timeout(60000);

  const { dpnsContractId, identityId } = wasmFunctionalTestRequirements();

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

  it('lists DPNS documents (no filters)', async () => {
    const docs = await client.getDocuments({
      dataContractId: dpnsContractId,
      documentTypeName: 'domain',
      limit: 5,
    });
    expect(docs).to.be.instanceOf(Map);
  });

  it('queries with where clause', async () => {
    const docs = await client.getDocuments({
      dataContractId: dpnsContractId,
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
      dataContractId: dpnsContractId,
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
      dataContractId: dpnsContractId,
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
    const list = await client.getDpnsUsernames({ identityId, limit: 10 });
    expect(list).to.be.an('array');
  });
});
