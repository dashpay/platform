const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');
const identityCreateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityCreate.json');
const identityTopUpTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityTopUp.json');
const identityUpdateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityUpdate.json');
const dataContractCreateTransitionSchema = require('@dashevo/dpp/schema/dataContract/stateTransition/dataContractCreate.json');
const documentsBatchTransitionSchema = require('@dashevo/dpp/schema/document/stateTransition/documentsBatch.json');
const dataContractMetaSchema = require('@dashevo/dpp/schema/dataContract/dataContractMeta.json');
const getBiggestPossibleIdentity = require('@dashevo/dpp/lib/identity/getBiggestPossibleIdentity');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

function createDataContractDocuments() {
  const name = new Array(62).fill('a').join('');

  const properties = {};
  for (let i = 0; i < dataContractMetaSchema.$defs.documentProperties.maxProperties / 20; i++) {
    properties[`${i}${name}`.slice(0, 62)] = {
      type: 'string',
    };
  }

  const documents = {};

  for (let i = 0; i < dataContractMetaSchema.properties.documents.maxProperties / 20; i++) {
    const indices = Object.keys(properties).map((propertyName) => ({
      properties: [{ [propertyName]: 'asc' },
      ],
      unique: false,
    }));

    documents[`${i}${name}`.slice(0, 62)] = {
      type: 'object',
      properties,
      additionalProperties: false,
      indices,
    };
  }

  return documents;
}

describe('checkFeePrediction', () => {
  let dpp;
  let container;
  let stateRepository;
  let instantAssetLockProof;
  let identity;
  let groveDBStore;
  let initialAppHash;

  beforeEach(async function beforeEach() {
    container = await createTestDIContainer();

    const blockExecutionContext = container.resolve('blockExecutionContext');
    blockExecutionContext.getHeader = this.sinon.stub().returns(
      { time: { seconds: 1651585250 } },
    );

    dpp = container.resolve('dpp');

    stateRepository = container.resolve('stateRepository');
    groveDBStore = container.resolve('groveDBStore');

    const createInitialStateStructure = container.resolve('createInitialStateStructure');
    await createInitialStateStructure();

    instantAssetLockProof = getInstantAssetLockProofFixture();

    const publicKeys = [];
    for (let i = 0; i < identityCreateTransitionSchema.properties.publicKeys.maxItems; i++) {
      publicKeys.push(new IdentityPublicKey({
        id: i,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: new PrivateKey().toPublicKey().toBuffer(),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
      }));
    }

    identity = getBiggestPossibleIdentity();
    identity.id = instantAssetLockProof.createIdentifier();
    identity.setAssetLockProof(instantAssetLockProof);
    identity.setPublicKeys(publicKeys);

    initialAppHash = await groveDBStore.getRootHash();
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  describe('Identity', () => {
    it('should check that IdentityCreateTransition predicted fee > real fee', async () => {
      const stateTransition = dpp.identity.createIdentityCreateTransition(identity);

      const signature = Buffer.alloc(identityCreateTransitionSchema.properties.signature.maxItems, '1');
      stateTransition.setSignature(signature);

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);

      await dpp.stateTransition.apply(stateTransition);

      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();

      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(initialAppHash).to.deep.equal(appHash);

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });

    it('should check that IdentityTopUpTransition predicted fee > real fee', async () => {
      await stateRepository.storeIdentity(identity);

      initialAppHash = await groveDBStore.getRootHash();

      const stateTransition = dpp.identity.createIdentityTopUpTransition(
        identity.getId(),
        instantAssetLockProof,
      );

      const signature = Buffer.alloc(identityTopUpTransitionSchema.properties.signature.maxItems, '1');
      stateTransition.setSignature(signature);

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });

    it('should check that IdentityUpdateTransition predicted fee > real fee', async () => {
      await stateRepository.storeIdentity(identity);

      initialAppHash = await groveDBStore.getRootHash();

      const newPublicKeys = [];

      for (let i = 0; i < identityUpdateTransitionSchema.properties.addPublicKeys.maxItems; i++) {
        newPublicKeys.push(
          new IdentityPublicKey({
            id: i,
            type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
            data: new PrivateKey().toPublicKey().toBuffer(),
            purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
            securityLevel: IdentityPublicKey.SECURITY_LEVELS.SECURITY_LEVELS,
            readOnly: false,
          }),
        );
      }

      const stateTransition = dpp.identity.createIdentityUpdateTransition(
        identity,
        {
          add: newPublicKeys,
          disable: newPublicKeys,
        },
      );

      const signature = Buffer.alloc(identityTopUpTransitionSchema.properties.signature.maxItems, '255');
      stateTransition.setSignature(signature);
      stateTransition.setSignaturePublicKeyId(Number.MAX_VALUE);
      stateTransition.setPublicKeysDisabledAt(new Date(Number.MAX_VALUE));

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();
      expect(initialAppHash).to.deep.equal(appHash);

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });
  });

  describe('DataContract', () => {
    let dataContract;

    beforeEach(async () => {
      const documents = createDataContractDocuments();

      dataContract = dpp.dataContract.create(identity.getId(), documents);
    });

    it('should check that DataContractCreate predicted fee > real fee', async () => {
      const stateTransition = dpp.dataContract.createDataContractCreateTransition(dataContract);

      stateTransition.setSignaturePublicKeyId(Number.MAX_VALUE);
      const signature = Buffer.alloc(dataContractCreateTransitionSchema.properties.signature.maxItems, '1');
      stateTransition.setSignature(signature);

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();
      expect(initialAppHash).to.deep.equal(appHash);

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });

    it('should check that DataContractUpdate predicted fee > real fee', async () => {
      const stateTransition = dpp.dataContract.createDataContractUpdateTransition(dataContract);
      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();
      expect(initialAppHash).to.deep.equal(appHash);

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });
  });

  describe('Document', () => {
    let documents;
    let dataContract;

    beforeEach(async () => {
      // в 1 индекс все поля(100 штук) уникальные
      const dataContractDocuments = createDataContractDocuments();
      dataContract = dpp.dataContract.create(identity.getId(), dataContractDocuments);
      documents = [];

      let i = 0;
      for (const documentType of Object.keys(dataContractDocuments)) {
        const data = {};

        for (const propertyName of Object.keys(dataContractDocuments[documentType].properties)) {
          data[propertyName] = new Array(63).fill('d').join('');
        }

        const document = dpp.document.create(
          dataContract,
          identity.getId(),
          documentType,
          data,
        );

        documents.push(document);

        i += 1;

        if (i === 10) {
          break;
        }
      }
    });

    it('should check that DocumentsBatchTransition create predicted fee > real fee', async () => {
      const stateTransition = dpp.document.createStateTransition({
        create: documents,
      });

      stateTransition.setSignaturePublicKeyId(Number.MAX_VALUE);
      const signature = Buffer.alloc(documentsBatchTransitionSchema.properties.signature.maxItems, '1');
      stateTransition.setSignature(signature);

      const executionContext = stateTransition.getExecutionContext();
      await stateRepository.storeDataContract(dataContract, executionContext);

      initialAppHash = await groveDBStore.getRootHash();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });

    it('should check that DocumentsBatchTransition update predicted fee > real fee', async () => {
      // const documentToReplace = documents[0];
      // documentToReplace.setData({ name: 'newName' });

      const createStateTransition = dpp.document.createStateTransition({
        create: documents,
      });

      const stateTransition = dpp.document.createStateTransition({
        replace: documents,
      });

      const executionContext = stateTransition.getExecutionContext();
      await stateRepository.storeDataContract(dataContract, executionContext);

      await dpp.stateTransition.validateState(createStateTransition);
      await dpp.stateTransition.apply(createStateTransition);

      initialAppHash = await groveDBStore.getRootHash();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });

    it('should check that DocumentsBatchTransition delete predicted fee > real fee', async () => {
      const createStateTransition = dpp.document.createStateTransition({
        create: documents,
      });

      const stateTransition = dpp.document.createStateTransition({
        delete: documents,
      });

      const executionContext = stateTransition.getExecutionContext();
      await stateRepository.storeDataContract(dataContract, executionContext);

      await dpp.stateTransition.validateState(createStateTransition);
      await dpp.stateTransition.apply(createStateTransition);

      initialAppHash = await groveDBStore.getRootHash();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expect(dryRunOperations.length).to.be.equal(actualOperations.length);

      dryRunOperations.forEach((dryRunOperation, i) => {
        expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
        expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
        expect(dryRunOperation.processingCost || 0)
          .to.be.gte(actualOperations[i].processingCost || 0);
      });
    });
  });
});
