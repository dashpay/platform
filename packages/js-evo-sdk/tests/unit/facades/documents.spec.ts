import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('DocumentsFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;
  let document: wasmSDKPackage.Document;
  let identityKey: wasmSDKPackage.IdentityPublicKey;
  let signer: wasmSDKPackage.IdentitySigner;
  const tokenPaymentInfo = {
    paymentTokenContractId: 'BpJvvpPiR2obh7ueZixjtYXsmWQdgJhiZtQJWjD7Ruus',
    tokenContractPosition: 0,
    minimumTokenCost: BigInt(10),
    maximumTokenCost: BigInt(25),
    gasFeesPaidBy: 'PreferContractOwner',
  };

  // Stub references for type-safe assertions
  let getDocumentsStub: SinonStub;
  let getDocumentsWithProofInfoStub: SinonStub;
  let getDocumentHistoryStub: SinonStub;
  let getDocumentHistoryWithProofInfoStub: SinonStub;
  let getDocumentStub: SinonStub;
  let getDocumentWithProofInfoStub: SinonStub;
  let documentCreateStub: SinonStub;
  let documentReplaceStub: SinonStub;
  let documentDeleteStub: SinonStub;
  let documentTransferStub: SinonStub;
  let documentPurchaseStub: SinonStub;
  let documentSetPriceStub: SinonStub;
  let getDocumentsCountStub: SinonStub;
  let getDocumentsCountWithProofInfoStub: SinonStub;
  let getDocumentsSumStub: SinonStub;
  let getDocumentsSumWithProofInfoStub: SinonStub;
  let getDocumentsAverageStub: SinonStub;
  let getDocumentsAverageWithProofInfoStub: SinonStub;
  let getDocumentsRankedStub: SinonStub;
  let getDocumentsRankedWithProofInfoStub: SinonStub;
  let getDocumentsHavingStub: SinonStub;
  let getDocumentsHavingWithProofInfoStub: SinonStub;

  const emptyRankedResult = {
    startingRank: BigInt(0),
    entries: [],
    aggregate: 'avg',
    groupBy: 'restaurantId',
    valueScale: BigInt(1),
  };
  const emptyHavingResult = {
    entries: [],
    aggregate: 'count',
    groupBy: 'hashtag',
    valueScale: BigInt(1),
  };

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    document = Object.create(wasmSDKPackage.Document.prototype);
    identityKey = Object.create(wasmSDKPackage.IdentityPublicKey.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    getDocumentsStub = this.sinon.stub(wasmSdk, 'getDocuments').resolves(new Map());
    getDocumentsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDocumentsWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getDocumentHistoryStub = this.sinon.stub(wasmSdk, 'getDocumentHistory').resolves(new Map());
    getDocumentHistoryWithProofInfoStub = this.sinon.stub(
      wasmSdk,
      'getDocumentHistoryWithProofInfo',
    ).resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getDocumentStub = this.sinon.stub(wasmSdk, 'getDocument').resolves(document);
    getDocumentWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDocumentWithProofInfo').resolves({
      data: document,
      proof: {},
      metadata: {},
    });

    // Stub transition methods
    documentCreateStub = this.sinon.stub(wasmSdk, 'documentCreate').resolves();
    documentReplaceStub = this.sinon.stub(wasmSdk, 'documentReplace').resolves();
    documentDeleteStub = this.sinon.stub(wasmSdk, 'documentDelete').resolves();
    documentTransferStub = this.sinon.stub(wasmSdk, 'documentTransfer').resolves();
    documentPurchaseStub = this.sinon.stub(wasmSdk, 'documentPurchase').resolves();
    documentSetPriceStub = this.sinon.stub(wasmSdk, 'documentSetPrice').resolves();

    // Stub aggregate query methods
    getDocumentsCountStub = this.sinon.stub(wasmSdk, 'getDocumentsCount').resolves(new Map());
    getDocumentsCountWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDocumentsCountWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getDocumentsSumStub = this.sinon.stub(wasmSdk, 'getDocumentsSum').resolves(new Map());
    getDocumentsSumWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDocumentsSumWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getDocumentsAverageStub = this.sinon.stub(wasmSdk, 'getDocumentsAverage').resolves(new Map());
    getDocumentsAverageWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDocumentsAverageWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });

    // Stub ranked / having-range query methods
    getDocumentsRankedStub = this.sinon.stub(wasmSdk, 'getDocumentsRanked').resolves(emptyRankedResult);
    getDocumentsRankedWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDocumentsRankedWithProofInfo').resolves({
      data: emptyRankedResult,
      proof: {},
      metadata: {},
    });
    getDocumentsHavingStub = this.sinon.stub(wasmSdk, 'getDocumentsHaving').resolves(emptyHavingResult);
    getDocumentsHavingWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDocumentsHavingWithProofInfo').resolves({
      data: emptyHavingResult,
      proof: {},
      metadata: {},
    });
  });

  describe('query()', () => {
    it('should fetch documents matching criteria', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'note',
        where: [['authorId', '==', '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS']],
        orderBy: [['createdAt', 'desc']],
        limit: 10,
      };

      await client.documents.query(query);

      expect(getDocumentsStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('queryWithProof()', () => {
    it('should fetch documents with proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'note',
      };

      await client.documents.queryWithProof(query);

      expect(getDocumentsWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('history()', () => {
    it('should fetch document history', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'note',
        documentId: '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah',
        startAtMs: 1000,
        limit: 10,
        offset: 1,
      };

      await client.documents.history(query);

      expect(getDocumentHistoryStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('historyWithProof()', () => {
    it('should fetch document history with proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'note',
        documentId: '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah',
      };

      await client.documents.historyWithProof(query);

      expect(getDocumentHistoryWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('get()', () => {
    it('should fetch a single document by ID', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
      const documentTypeName = 'note';
      const documentId = '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah';

      await client.documents.get(contractId, documentTypeName, documentId);

      expect(getDocumentStub)
        .to.be.calledOnceWithExactly(contractId, documentTypeName, documentId);
    });
  });

  describe('getWithProof()', () => {
    it('should fetch a single document with proof', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
      const documentTypeName = 'note';
      const documentId = '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah';

      await client.documents.getWithProof(contractId, documentTypeName, documentId);

      expect(getDocumentWithProofInfoStub)
        .to.be.calledOnceWithExactly(contractId, documentTypeName, documentId);
    });
  });

  describe('create()', () => {
    it('should create a new document', async () => {
      const options = {
        document,
        identityKey,
        signer,
        tokenPaymentInfo,
      };

      await client.documents.create(options);

      expect(documentCreateStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('replace()', () => {
    it('should replace an existing document', async () => {
      const options = {
        document,
        identityKey,
        signer,
        tokenPaymentInfo,
        settings: { retries: 3 },
      };

      await client.documents.replace(options);

      expect(documentReplaceStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('delete()', () => {
    it('should delete a document', async () => {
      const options = {
        document,
        identityKey,
        signer,
        tokenPaymentInfo,
      };

      await client.documents.delete(options);

      expect(documentDeleteStub).to.be.calledOnceWithExactly(options);
    });

    it('should accept document identifiers instead of Document instance', async () => {
      const options = {
        document: {
          id: '4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah',
          ownerId: '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
          dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
          documentTypeName: 'note',
        },
        identityKey,
        signer,
        tokenPaymentInfo,
      };

      await client.documents.delete(options);

      expect(documentDeleteStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('transfer()', () => {
    it('should transfer document ownership to another identity', async () => {
      const recipientId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';
      const options = {
        document,
        recipientId,
        identityKey,
        signer,
        tokenPaymentInfo,
      };

      await client.documents.transfer(options);

      expect(documentTransferStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('purchase()', () => {
    it('should purchase a document from another identity', async () => {
      const buyerId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';
      const options = {
        document,
        buyerId,
        price: BigInt(1000000), // 1M credits
        identityKey,
        signer,
        tokenPaymentInfo,
      };

      await client.documents.purchase(options);

      expect(documentPurchaseStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('setPrice()', () => {
    it('should set a price on a document for sale', async () => {
      const options = {
        document,
        price: BigInt(5000000), // 5M credits
        identityKey,
        signer,
        tokenPaymentInfo,
      };

      await client.documents.setPrice(options);

      expect(documentSetPriceStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('count()', () => {
    it('should count documents matching a query', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'grade',
      };

      await client.documents.count(query);

      expect(getDocumentsCountStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('countWithProof()', () => {
    it('should count documents and return proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'grade',
        where: [['class', '==', 'CS101']],
      };

      await client.documents.countWithProof(query);

      expect(getDocumentsCountWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('sum()', () => {
    it('should aggregate a summable property across matching documents', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'grade',
        where: [['semester', '==', 20251]],
      };
      const sumProperty = 'score';

      await client.documents.sum(query, sumProperty);

      expect(getDocumentsSumStub).to.be.calledOnceWithExactly(query, sumProperty);
    });
  });

  describe('sumWithProof()', () => {
    it('should aggregate a summable property with proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'grade',
      };
      const sumProperty = 'score';

      await client.documents.sumWithProof(query, sumProperty);

      expect(getDocumentsSumWithProofInfoStub).to.be.calledOnceWithExactly(query, sumProperty);
    });
  });

  describe('average()', () => {
    it('should return the (count, sum) pair for a summable property', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'grade',
        groupBy: ['class'],
      };
      const averageProperty = 'score';

      await client.documents.average(query, averageProperty);

      expect(getDocumentsAverageStub).to.be.calledOnceWithExactly(query, averageProperty);
    });
  });

  describe('averageWithProof()', () => {
    it('should return the (count, sum) pair with proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'grade',
        where: [['class', '==', 'CS101']],
        groupBy: ['semester'],
      };
      const averageProperty = 'score';

      await client.documents.averageWithProof(query, averageProperty);

      expect(getDocumentsAverageWithProofInfoStub).to.be.calledOnceWithExactly(query, averageProperty);
    });
  });

  describe('ranked()', () => {
    it('should rank groups by an aggregate', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'review',
        groupBy: 'restaurantId',
        aggregate: { type: 'avg', property: 'grade' },
        limit: 3,
      };

      await client.documents.ranked(query);

      expect(getDocumentsRankedStub).to.be.calledOnceWithExactly(query);
    });

    it('should pass through the offset that selects a single rank', async () => {
      // "The 5th best": skip the four above it, take one.
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'review',
        groupBy: 'restaurantId',
        aggregate: { type: 'avg', property: 'grade' },
        limit: 1,
        offset: 4,
      };

      await client.documents.ranked(query);

      expect(getDocumentsRankedStub).to.be.calledOnceWithExactly(query);
    });

    it('should pass through the equality pins of a compound ranked index', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'grade',
        groupBy: 'class',
        aggregate: { type: 'count' },
        where: [['country', '==', 'DE']],
        direction: 'asc',
        limit: 10,
      };

      await client.documents.ranked(query);

      expect(getDocumentsRankedStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('rankedWithProof()', () => {
    it('should rank groups with proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'review',
        groupBy: 'restaurantId',
        aggregate: { type: 'count' },
        limit: 5,
      };

      await client.documents.rankedWithProof(query);

      expect(getDocumentsRankedWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('having()', () => {
    it('should bound groups by their aggregate', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'post',
        groupBy: 'hashtag',
        aggregate: { type: 'count' },
        having: { operator: '>', value: 100 },
        direction: 'desc',
        limit: 100,
      };

      await client.documents.having(query);

      expect(getDocumentsHavingStub).to.be.calledOnceWithExactly(query);
    });

    it('should pass through a two-operand between bound', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'tip',
        groupBy: 'recipientId',
        aggregate: { type: 'sum', property: 'amount' },
        having: { operator: 'between', value: [1000, 5000] },
        limit: 25,
      };

      await client.documents.having(query);

      expect(getDocumentsHavingStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('havingWithProof()', () => {
    it('should bound groups with proof metadata', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        documentTypeName: 'post',
        groupBy: 'hashtag',
        aggregate: { type: 'count' },
        having: { operator: '>=', value: BigInt(1) },
        limit: 10,
      };

      await client.documents.havingWithProof(query);

      expect(getDocumentsHavingWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });
});
