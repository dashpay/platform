const fs = require('fs');

const { expect, use } = require('chai');
use(require('dirty-chai'));

const Document = require('@dashevo/dpp/lib/document/Document');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const {
  expectFeeResult,
} = require('./utils');

const Drive = require('../Drive');

const FeeResult = require('../FeeResult');

const TEST_DATA_PATH = './test_data';

describe('Drive', () => {
  let drive;
  let dataContract;
  let identity;
  let blockInfo;
  let documents;
  let initialRootHash;

  beforeEach(async () => {
    drive = new Drive(TEST_DATA_PATH, {
      dataContractsGlobalCacheSize: 500,
      dataContractsBlockCacheSize: 500,
    });

    dataContract = getDataContractFixture();
    identity = getIdentityFixture();

    blockInfo = {
      height: 1,
      epoch: 1,
      timeMs: new Date().getTime(),
    };

    documents = getDocumentsFixture(dataContract);
    initialRootHash = await drive.getGroveDB().getRootHash();
  });

  afterEach(async () => {
    await drive.close();

    fs.rmSync(TEST_DATA_PATH, { recursive: true });
  });

  describe('#createInitialStateStructure', () => {
    it('should create initial tree structure', async () => {
      const result = await drive.createInitialStateStructure();

      // eslint-disable-next-line no-unused-expressions
      expect(result).to.be.undefined;
    });
  });

  describe('#fetchContract', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should return null if contract not exists', async () => {
      const result = await drive.fetchContract(Buffer.alloc(32), blockInfo.epoch);

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [fetchedDataContract, feeResult] = result;

      expect(fetchedDataContract).to.be.null();

      expect(feeResult).to.be.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });

    it('should return contract if contract is present', async () => {
      await drive.createContract(dataContract, blockInfo);

      const result = await drive.fetchContract(dataContract.getId(), blockInfo.epoch);

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [fetchedDataContract, feeResult] = result;

      expect(fetchedDataContract.toBuffer()).to.deep.equal(dataContract.toBuffer());

      expect(feeResult).to.be.an.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });

    it('should return contract without fee result if epoch is not passed', async () => {
      await drive.createContract(dataContract, blockInfo);

      const result = await drive.fetchContract(dataContract.getId());

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(1);

      const [fetchedDataContract] = result;

      expect(fetchedDataContract.toBuffer()).to.deep.equal(dataContract.toBuffer());
    });
  });

  describe('#createContract', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should create contract', async () => {
      const result = await drive.createContract(dataContract, blockInfo);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
    });

    it('should not create contract with dry run flag', async () => {
      const result = await drive.createContract(dataContract, blockInfo, undefined, true);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#updateContract', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.createContract(dataContract, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();

      dataContract.setDocumentSchema('newDocumentType', {
        type: 'object',
        properties: {
          newProperty: {
            type: 'string',
          },
        },
        additionalProperties: false,
      });
    });

    it('should update existing contract', async () => {
      const result = await drive.updateContract(dataContract, blockInfo);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
    });

    it('should not create contract with dry run flag', async () => {
      const result = await drive.updateContract(dataContract, blockInfo, undefined, true);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#createDocument', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.createContract(dataContract, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    context('without indices', () => {
      it('should create a document', async () => {
        const documentWithoutIndices = documents[0];

        const result = await drive.createDocument(documentWithoutIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    context('with indices', () => {
      it('should create a document', async () => {
        const documentWithIndices = documents[3];

        const result = await drive.createDocument(documentWithIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    it('should not create a document with dry run flag', async () => {
      const documentWithoutIndices = documents[0];

      const result = await drive.createDocument(documentWithoutIndices, blockInfo, undefined, true);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#updateDocument', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.createContract(dataContract, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    context('without indices', () => {
      it('should update a document', async () => {
        // Create a document
        const documentWithoutIndices = documents[0];

        await drive.createDocument(documentWithoutIndices, blockInfo);

        // Update the document
        documentWithoutIndices.set('name', 'Boooooooooooob');

        const result = await drive.updateDocument(documentWithoutIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    context('with indices', () => {
      it('should update the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices, blockInfo);

        // Update the document
        documentWithIndices.set('firstName', 'Bob');

        const result = await drive.updateDocument(documentWithIndices, blockInfo);

        expectFeeResult(result);

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    it('should not update a document with dry run flag', async () => {
      const documentWithoutIndices = documents[0];

      documentWithoutIndices.set('name', 'Boooooooooooooooooooooob');

      const result = await drive.updateDocument(documentWithoutIndices, blockInfo, undefined, true);

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.be.greaterThan(0);
      expect(result.storageFee).to.be.greaterThan(0, 'storage fee must be higher than 0');

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#deleteDocument', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.createContract(dataContract, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    context('without indices', () => {
      it('should delete the document', async () => {
        // Create a document
        const documentWithoutIndices = documents[3];

        await drive.createDocument(documentWithoutIndices, blockInfo);

        initialRootHash = await drive.getGroveDB().getRootHash();

        const result = await drive.deleteDocument(
          dataContract.getId(),
          documentWithoutIndices.getType(),
          documentWithoutIndices.getId(),
          blockInfo,
        );

        expect(result).to.be.an.instanceOf(FeeResult);

        expect(result.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
        expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    context('with indices', () => {
      it('should delete the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices, blockInfo);

        initialRootHash = await drive.getGroveDB().getRootHash();

        const result = await drive.deleteDocument(
          dataContract.getId(),
          documentWithIndices.getType(),
          documentWithIndices.getId(),
          blockInfo,
        );

        expect(result).to.be.an.instanceOf(FeeResult);

        expect(result.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
        expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

        expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
      });
    });

    it('should not delete the document with dry run flag', async () => {
      // Create a document
      const documentWithoutIndices = documents[3];

      await drive.createDocument(documentWithoutIndices, blockInfo);

      initialRootHash = await drive.getGroveDB().getRootHash();

      const result = await drive.deleteDocument(
        dataContract.getId(),
        documentWithoutIndices.getType(),
        documentWithoutIndices.getId(),
        blockInfo,
        undefined,
        true,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#queryDocuments', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.createContract(dataContract, blockInfo);
    });

    it('should query existing documents', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockInfo)),
      );

      // eslint-disable-next-line no-unused-vars
      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', undefined, {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(1);
      expect(fetchedDocuments[0]).to.be.an.instanceOf(Document);
      expect(fetchedDocuments[0].toObject()).to.deep.equal(documents[4].toObject());

      // costs are not calculating without block info
      expect(processingCost).to.be.equal(0);
    });

    it('should query existing documents again', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockInfo)),
      );

      // eslint-disable-next-line no-unused-vars
      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', blockInfo.epoch, {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(1);
      expect(fetchedDocuments[0]).to.be.an.instanceOf(Document);
      expect(fetchedDocuments[0].toObject()).to.deep.equal(documents[4].toObject());

      expect(processingCost).to.be.greaterThan(0);
    });

    it('should return empty array if documents are not exist', async () => {
      // eslint-disable-next-line no-unused-vars
      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', blockInfo.epoch, {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(0);
      expect(processingCost).to.be.greaterThan(0);
    });
  });

  describe('#proveDocumentsQuery', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      await drive.createContract(dataContract, blockInfo);
    });

    it('should query existing documents', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockInfo)),
      );

      const result = await drive.proveDocumentsQuery(dataContract, 'indexedDocument', {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(result).to.have.lengthOf(2);

      const [proofs, processingCost] = result;
      expect(proofs).to.be.an.instanceOf(Buffer);
      expect(proofs.length).to.be.greaterThan(0);

      // TODO: We do not calculating processing costs for poofs yet
      expect(processingCost).to.equal(0);
    });
  });

  describe('#insertIdentity', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should create identity if not exists', async () => {
      const result = await drive.insertIdentity(identity, blockInfo);

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.not.deep.equals(initialRootHash);
    });
  });

  describe('#fetchIdentity', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should return null if identity not exists', async () => {
      const result = await drive.fetchIdentity(Buffer.alloc(32));

      expect(result).to.be.null();
    });

    it('should return identity if it is present', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const result = await drive.fetchIdentity(identity.getId());

      expect(result.toBuffer()).to.deep.equal(identity.toBuffer());
    });
  });

  describe('#fetchIdentityWithCosts', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should return null if contract not exists', async () => {
      const result = await drive.fetchIdentityWithCosts(Buffer.alloc(32), blockInfo.epoch);

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [fetchedIdentity, feeResult] = result;

      expect(fetchedIdentity).to.be.null();

      expect(feeResult).to.be.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });

    it('should return contract if contract is present', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const result = await drive.fetchIdentityWithCosts(identity.getId(), blockInfo.epoch);

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [fetchedIdentity, feeResult] = result;

      expect(fetchedIdentity.toBuffer()).to.deep.equal(identity.toBuffer());

      expect(feeResult).to.be.an.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });
  });

  describe('#fetchIdentityBalance', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should return null if identity not exists', async () => {
      const result = await drive.fetchIdentityBalance(Buffer.alloc(32));

      expect(result).to.be.null();
    });

    it('should return balance if identity is present', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const balance = await drive.fetchIdentityBalance(identity.getId());

      expect(balance).to.deep.equal(identity.getBalance());
    });
  });

  describe('#fetchIdentityBalanceWithCosts', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should return null if identity not exists', async () => {
      const result = await drive.fetchIdentityBalanceWithCosts(Buffer.alloc(32), blockInfo);

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [balance, feeResult] = result;

      expect(balance).to.be.null();

      expect(feeResult).to.be.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });

    it('should return contract if identity is present', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const result = await drive.fetchIdentityBalanceWithCosts(identity.getId(), blockInfo);

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [balance, feeResult] = result;

      expect(balance).to.equal(identity.getBalance());

      expect(feeResult).to.be.an.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });
  });

  describe('#fetchIdentityBalanceIncludeDebtWithCosts', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should return null if identity not exists', async () => {
      const result = await drive.fetchIdentityBalanceIncludeDebtWithCosts(
        Buffer.alloc(32),
        blockInfo,
      );

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [balance, feeResult] = result;

      expect(balance).to.be.null();

      expect(feeResult).to.be.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });

    it('should return balance if identity is present', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const result = await drive.fetchIdentityBalanceIncludeDebtWithCosts(
        identity.getId(),
        blockInfo,
      );

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.have.lengthOf(2);

      const [balance, feeResult] = result;

      expect(balance).to.equal(identity.getBalance());

      expect(feeResult).to.be.an.instanceOf(FeeResult);

      expect(feeResult.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(feeResult.storageFee).to.be.equal(0, 'storage fee must be equal to 0');
    });
  });

  describe('#addToIdentityBalance', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should add to balance', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const amount = 100;

      const result = await drive.addToIdentityBalance(
        identity.getId(),
        amount,
        blockInfo,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      const fetchedIdentity = await drive.fetchIdentity(identity.getId());

      expect(fetchedIdentity.getBalance()).to.equals(identity.getBalance() + amount);
    });

    it('should not update state with dry run', async () => {
      initialRootHash = await drive.getGroveDB().getRootHash();

      const result = await drive.addToIdentityBalance(
        identity.getId(),
        100,
        blockInfo,
        false,
        true,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#removeFromIdentitiyBalance', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should remove from balance', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const amount = 2;

      const result = await drive.removeFromIdentityBalance(
        identity.getId(),
        amount,
        blockInfo,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      const fetchedIdentity = await drive.fetchIdentity(identity.getId());

      expect(fetchedIdentity.getBalance()).to.equals(
        identity.getBalance() - amount,
      );
    });

    it('should not update state with dry run', async () => {
      initialRootHash = await drive.getGroveDB().getRootHash();

      const amount = 2;

      const result = await drive.removeFromIdentityBalance(
        identity.getId(),
        amount,
        blockInfo,
        false,
        true,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#applyFeesToIdentityBalance', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should change balance according to provided fees', async () => {
      blockInfo.epoch = 3;

      await drive.insertIdentity(identity, blockInfo);

      const feeResult = FeeResult.create(10000, 10, [{
        identifier: identity.getId().toBuffer(),
        creditsPerEpoch: { 0: 1000000 },
      }]);

      const result = await drive.applyFeesToIdentityBalance(
        identity.getId(),
        feeResult,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      const fetchedIdentity = await drive.fetchIdentity(identity.getId());

      expect(fetchedIdentity.getBalance()).to.not.equals(
        identity.getBalance(),
      );
    });
  });

  describe('#addToSystemCredits', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should add to system credits', async () => {
      await drive.addToSystemCredits(
        100,
      );
    });
  });

  describe('#fetchIdentitiesByPublicKeyHashes', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should return empty array if identity not exists', async () => {
      const result = await drive.fetchIdentitiesByPublicKeyHashes([Buffer.alloc(20)]);

      expect(result).to.be.instanceOf(Array);
      expect(result).to.be.empty();
    });

    it('should return identities if it is present', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const result = await drive.fetchIdentitiesByPublicKeyHashes(
        identity.getPublicKeys().map((k) => k.hash()),
      );

      expect(result).to.deep.equal(identity.getPublicKeys().map(() => identity));
    });
  });

  describe('#addKeysToIdentity', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should add keys to identity', async () => {
      const keysToAdd = [identity.getPublicKeys().pop()];

      await drive.insertIdentity(identity, blockInfo);

      const result = await drive.addKeysToIdentity(
        identity.getId(),
        keysToAdd,
        blockInfo,
      );

      expectFeeResult(result);

      const fetchedIdentity = await drive.fetchIdentity(identity.getId());

      expect(fetchedIdentity.getPublicKeys()).to.have.lengthOf(
        identity.getPublicKeys().length + keysToAdd.length,
      );
    });

    it('should not update state with dry run', async () => {
      initialRootHash = await drive.getGroveDB().getRootHash();

      const keysToAdd = [identity.getPublicKeys().pop()];

      const result = await drive.addKeysToIdentity(
        identity.getId(),
        keysToAdd,
        blockInfo,
        false,
        true,
      );

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#disableIdentityKeys', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should disable specified identity keys', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const keyIds = identity.getPublicKeys().map((key) => key.getId());
      const disableAt = Date.now();

      const result = await drive.disableIdentityKeys(
        identity.getId(),
        keyIds,
        disableAt,
        blockInfo,
      );

      expectFeeResult(result);

      const fetchedIdentity = await drive.fetchIdentity(identity.getId());

      expect(fetchedIdentity.getPublicKeys()).to.have.lengthOf(identity.getPublicKeys().length);

      fetchedIdentity.getPublicKeys().forEach((key) => {
        expect(key.getDisabledAt()).to.equals(disableAt);
      });
    });

    it('should not update state with dry run', async () => {
      const keyIds = identity.getPublicKeys().map((key) => key.getId());
      const disableAt = Date.now();

      const result = await drive.disableIdentityKeys(
        identity.getId(),
        keyIds,
        disableAt,
        blockInfo,
        false,
        true,
      );

      expectFeeResult(result);

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#updateIdentityRevision', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();

      initialRootHash = await drive.getGroveDB().getRootHash();
    });

    it('should update identity revision', async () => {
      await drive.insertIdentity(identity, blockInfo);

      const revision = 2;

      const result = await drive.updateIdentityRevision(
        identity.getId(),
        revision,
        blockInfo,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      const fetchedIdentity = await drive.fetchIdentity(identity.getId());

      expect(fetchedIdentity.getRevision()).to.equals(revision);
    });

    it('should not update state with dry run', async () => {
      const result = await drive.updateIdentityRevision(
        identity.getId(),
        2,
        blockInfo,
        false,
        true,
      );

      expect(result).to.be.an.instanceOf(FeeResult);

      expect(result.processingFee).to.greaterThan(0, 'processing fee must be higher than 0');
      expect(result.storageFee).to.be.equal(0, 'storage fee must be equal to 0');

      expect(await drive.getGroveDB().getRootHash()).to.deep.equals(initialRootHash);
    });
  });

  describe('#fetchLatestWithdrawalTransactionIndex', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should return 0 on the first call', async () => {
      const result = await drive.fetchLatestWithdrawalTransactionIndex();

      expect(result).to.equal(0);
    });
  });

  describe('#enqueueWithdrawalTransaction', () => {
    beforeEach(async () => {
      await drive.createInitialStateStructure();
    });

    it('should enqueue withdrawal transaction into the queue', async () => {
      await drive.enqueueWithdrawalTransaction(1, Buffer.alloc(32, 1));

      const result = await drive.fetchLatestWithdrawalTransactionIndex();

      expect(result).to.equal(1);
    });
  });

  describe('ABCI', () => {
    describe('InitChain', () => {
      it('should successfully init chain', async () => {
        const request = {};

        const response = await drive.getAbci().initChain(request);

        expect(response).to.be.empty('object');
      });
    });

    describe('BlockBegin', () => {
      beforeEach(async () => {
        await drive.getAbci().initChain({});
      });

      it('should process a block without previous block time', async () => {
        const request = {
          blockHeight: 1,
          blockTimeMs: (new Date()).getTime(),
          proposerProTxHash: Buffer.alloc(32, 1),
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        };

        const response = await drive.getAbci().blockBegin(request);

        expect(response.unsignedWithdrawalTransactions).to.be.empty();
        expect(response.epochInfo).to.deep.equal({
          currentEpochIndex: 0,
          isEpochChange: true,
          previousEpochIndex: null,
        });
      });

      it('should process a block with previous block time', async () => {
        const blockTimeMs = (new Date()).getTime();

        await drive.getAbci().blockBegin({
          blockHeight: 1,
          blockTimeMs,
          proposerProTxHash: Buffer.alloc(32, 1),
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        });

        const response = await drive.getAbci().blockBegin({
          blockHeight: 2,
          blockTimeMs: blockTimeMs + 100,
          proposerProTxHash: Buffer.alloc(32, 1),
          previousBlockTimeMs: blockTimeMs,
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        });

        expect(response.unsignedWithdrawalTransactions).to.be.empty();
        expect(response.epochInfo).to.deep.equal({
          currentEpochIndex: 0,
          isEpochChange: false,
          previousEpochIndex: null,
        });
      });
    });

    describe('BlockEnd', () => {
      beforeEach(async () => {
        await drive.getAbci().initChain({});

        await drive.getAbci().blockBegin({
          blockHeight: 1,
          blockTimeMs: (new Date()).getTime(),
          proposerProTxHash: Buffer.alloc(32, 1),
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        });
      });

      it('should process a block', async () => {
        const request = {
          fees: {
            storageFee: 0,
            processingFee: 0,
            refundsPerEpoch: { },
          },
        };

        const response = await drive.getAbci().blockEnd(request);

        expect(response).to.have.property('proposersPaidCount');
        expect(response).to.have.property('paidEpochIndex');
        expect(response).to.have.property('refundedEpochsCount');
      });
    });

    describe('AfterFinalizeBlock', () => {
      beforeEach(async function beforeEach() {
        this.timeout(10000);

        await drive.getAbci().initChain({});

        await drive.getAbci().blockBegin({
          blockHeight: 1,
          blockTimeMs: (new Date()).getTime(),
          proposerProTxHash: Buffer.alloc(32, 1),
          validatorSetQuorumHash: Buffer.alloc(32, 2),
        });

        await drive.getAbci().blockEnd({
          fees: {
            storageFee: 0,
            processingFee: 0,
            refundsPerEpoch: {},
          },
        });
      });

      it('should process a block', async function it() {
        this.timeout(10000);

        const request = {
          updatedDataContractIds: [dataContract.getId()],
        };

        const response = await drive.getAbci().afterFinalizeBlock(request);

        expect(response).to.be.empty();
      });
    });
  });

  describe('.calculateStorageFeeDistributionAmountAndLeftovers', () => {
    it('should calculate amount and leftovers', () => {
      const result = Drive.calculateStorageFeeDistributionAmountAndLeftovers(1000, 1, 2);

      expect(result).to.be.an.instanceOf(Array);
      expect(result).to.be.lengthOf(2);

      const [amount, leftovers] = result;

      expect(amount).to.equals(556);
      expect(leftovers).to.equals(440);
    });
  });
});
