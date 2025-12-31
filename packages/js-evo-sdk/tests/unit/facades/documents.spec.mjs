import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('DocumentsFacade', () => {
  let wasmSdk;
  let client;
  let document;
  let identityKey;
  let signer;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    document = Object.create(wasmSDKPackage.Document.prototype);
    identityKey = Object.create(wasmSDKPackage.IdentityPublicKey.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    this.sinon.stub(wasmSdk, 'getDocuments').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getDocumentsWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getDocument').resolves(document);
    this.sinon.stub(wasmSdk, 'getDocumentWithProofInfo').resolves({
      data: document,
      proof: {},
      metadata: {},
    });

    // Stub transition methods
    this.sinon.stub(wasmSdk, 'documentCreate').resolves();
    this.sinon.stub(wasmSdk, 'documentReplace').resolves();
    this.sinon.stub(wasmSdk, 'documentDelete').resolves();
    this.sinon.stub(wasmSdk, 'documentTransfer').resolves();
    this.sinon.stub(wasmSdk, 'documentPurchase').resolves();
    this.sinon.stub(wasmSdk, 'documentSetPrice').resolves();
  });

  describe('Query Methods', () => {
    it('query() fetches documents matching criteria', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'note',
        where: [['authorId', '==', '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS']],
        orderBy: [['createdAt', 'desc']],
        limit: 10,
      };

      await client.documents.query(query);

      expect(wasmSdk.getDocuments).to.be.calledOnceWithExactly(query);
    });

    it('queryWithProof() fetches documents with proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'note',
      };

      await client.documents.queryWithProof(query);

      expect(wasmSdk.getDocumentsWithProofInfo).to.be.calledOnceWithExactly(query);
    });

    it('get() fetches a single document by ID', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
      const documentTypeName = 'note';
      const documentId = '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah';

      await client.documents.get(contractId, documentTypeName, documentId);

      expect(wasmSdk.getDocument).to.be.calledOnceWithExactly(contractId, documentTypeName, documentId);
    });

    it('getWithProof() fetches a single document with proof', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
      const documentTypeName = 'note';
      const documentId = '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah';

      await client.documents.getWithProof(contractId, documentTypeName, documentId);

      expect(wasmSdk.getDocumentWithProofInfo).to.be.calledOnceWithExactly(contractId, documentTypeName, documentId);
    });
  });

  describe('Transition Methods', () => {
    it('create() creates a new document', async () => {
      const options = {
        document,
        identityKey,
        signer,
      };

      await client.documents.create(options);

      expect(wasmSdk.documentCreate).to.be.calledOnceWithExactly(options);
    });

    it('replace() replaces an existing document', async () => {
      const options = {
        document,
        identityKey,
        signer,
        settings: { retries: 3 },
      };

      await client.documents.replace(options);

      expect(wasmSdk.documentReplace).to.be.calledOnceWithExactly(options);
    });

    it('delete() deletes a document', async () => {
      const options = {
        document,
        identityKey,
        signer,
      };

      await client.documents.delete(options);

      expect(wasmSdk.documentDelete).to.be.calledOnceWithExactly(options);
    });

    it('delete() accepts document identifiers instead of Document instance', async () => {
      const options = {
        document: {
          id: '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah',
          ownerId: '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
          dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
          documentTypeName: 'note',
        },
        identityKey,
        signer,
      };

      await client.documents.delete(options);

      expect(wasmSdk.documentDelete).to.be.calledOnceWithExactly(options);
    });

    it('transfer() transfers document ownership to another identity', async () => {
      const recipientId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';
      const options = {
        document,
        recipientId,
        identityKey,
        signer,
      };

      await client.documents.transfer(options);

      expect(wasmSdk.documentTransfer).to.be.calledOnceWithExactly(options);
    });

    it('purchase() purchases a document from another identity', async () => {
      const buyerId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';
      const options = {
        document,
        buyerId,
        price: BigInt(1000000), // 1M credits
        identityKey,
        signer,
      };

      await client.documents.purchase(options);

      expect(wasmSdk.documentPurchase).to.be.calledOnceWithExactly(options);
    });

    it('setPrice() sets a price on a document for sale', async () => {
      const options = {
        document,
        price: BigInt(5000000), // 5M credits
        identityKey,
        signer,
      };

      await client.documents.setPrice(options);

      expect(wasmSdk.documentSetPrice).to.be.calledOnceWithExactly(options);
    });
  });
});
