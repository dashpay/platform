import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Document Queries', function describeDocumentQueries() {
  this.timeout(60000);

  const { dpnsContractId, identityId } = wasmFunctionalTestRequirements();

  let client: sdk.WasmSdk;
  let builder: sdk.WasmSdkBuilder;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('getDocuments()', () => {
    it('should list DPNS documents with no filters', async () => {
      const docs = await client.getDocuments({
        dataContractId: dpnsContractId,
        documentTypeName: 'domain',
        limit: 5,
      });
      expect(docs).to.be.instanceOf(Map);
    });

    it('should query with where clause', async () => {
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

    it('should query with orderBy', async () => {
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

    it('should query with complex where and orderBy', async () => {
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
  });

  describe('getDocument()', () => {
    it('should handle invalid id gracefully', async () => {
      await expect(
        client.getDocument(dpnsContractId, 'domain', 'invalidDocumentId'),
      ).to.be.rejected();
    });
  });

  describe('getDpnsUsernames()', () => {
    it('should fetch usernames for a known identity and verify fields', async () => {
      const list = await client.getDpnsUsernames({ identityId, limit: 10 });
      expect(list).to.be.an('array');
    });
  });
});
