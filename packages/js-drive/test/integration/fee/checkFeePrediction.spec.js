const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');
const identityCreateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityCreate.json');
const identityUpdateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityUpdate.json');
const dataContractMetaSchema = require('@dashevo/dpp/schema/dataContract/dataContractMeta.json');
const getBiggestPossibleIdentity = require('@dashevo/dpp/lib/identity/getBiggestPossibleIdentity');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

function createDataContractDocuments() {
  const name = new Array(62).fill('a').join('');

  const properties = {};

  // we need to fit 16kb size limit
  for (let i = 0; i < dataContractMetaSchema.$defs.documentProperties.maxProperties; i++) {
    properties[`${name.slice(0, 4)}${i}`] = {
      type: 'string',
      maxLength: 62,
    };
  }

  const documents = {};

  for (let i = 0; i < dataContractMetaSchema.properties.documents.maxProperties; i++) {
    const indices = [{
      name: 'index1',
      properties: Object.keys(properties).slice(0, 10).map((propertyName) => ({
        [propertyName]:
      'asc',
      })),
      unique: true,
    }];

    documents[`${name}${i}`.slice(0, 62)] = {
      type: 'object',
      properties,
      additionalProperties: false,
      indices,
    };
  }

  return documents;
}

function expectFeeCalculationsAreValid(dryRunOperations, actualOperations) {
  expect(dryRunOperations.length).to.be.equal(actualOperations.length);

  dryRunOperations.forEach((dryRunOperation, i) => {
    expect(dryRunOperation.valueSize || 0).to.be.gte(actualOperations[i].valueSize || 0);
    expect(dryRunOperation.storageCost || 0).to.be.gte(actualOperations[i].storageCost || 0);
    expect(dryRunOperation.processingCost || 0)
      .to.be.gte(actualOperations[i].processingCost || 0);
  });
}

describe('checkFeePrediction', () => {
  let dpp;
  let container;
  let stateRepository;
  let instantAssetLockProof;
  let identity;
  let groveDBStore;
  let initialAppHash;
  let privateKeys;
  let assetLockPrivateKey;

  beforeEach(async function beforeEach() {
    assetLockPrivateKey = new PrivateKey();

    container = await createTestDIContainer();

    const blockExecutionContext = container.resolve('blockExecutionContext');
    blockExecutionContext.getHeader = this.sinon.stub().returns(
      { time: { seconds: new Date().getTime() / 1000 } },
    );

    dpp = container.resolve('dpp');

    stateRepository = container.resolve('stateRepository');

    stateRepository.verifyInstantLock = this.sinon.stub().resolves(true);

    groveDBStore = container.resolve('groveDBStore');

    const createInitialStateStructure = container.resolve('createInitialStateStructure');
    await createInitialStateStructure();

    instantAssetLockProof = getInstantAssetLockProofFixture(assetLockPrivateKey);

    const publicKeys = [];
    privateKeys = [];

    for (let i = 0; i < identityCreateTransitionSchema.properties.publicKeys.maxItems; i++) {
      const privateKey = new PrivateKey();

      privateKeys.push(privateKey);

      publicKeys.push(new IdentityPublicKey({
        id: i,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: privateKey.toPublicKey().toBuffer(),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: i === 0
          ? IdentityPublicKey.SECURITY_LEVELS.MASTER : IdentityPublicKey.SECURITY_LEVELS.HIGH,
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

      const publicKeys = stateTransition.getPublicKeys();

      for (let i = 0; i < publicKeys.length; i++) {
        await stateTransition.signByPrivateKey(
          privateKeys[i],
          IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        );
        publicKeys[i].setSignature(stateTransition.getSignature());
        stateTransition.setSignature(undefined);
      }

      await stateTransition.signByPrivateKey(
        assetLockPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      // validate state transition
      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);
      expect(validateBasicResult.isValid()).to.be.true();

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      // calculate predicted fee

      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);

      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();

      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(initialAppHash).to.deep.equal(appHash);

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
    });

    it('should check that IdentityTopUpTransition predicted fee > real fee', async () => {
      await stateRepository.storeIdentity(identity);

      initialAppHash = await groveDBStore.getRootHash();

      const stateTransition = dpp.identity.createIdentityTopUpTransition(
        identity.getId(),
        instantAssetLockProof,
      );

      await stateTransition.signByPrivateKey(
        assetLockPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      // validate state transition

      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);

      expect(validateBasicResult.isValid()).to.be.true();

      const executionContext = stateTransition.getExecutionContext();
      executionContext.enableDryRun();

      // calculate predicted fee
      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);

      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
    });

    it('should check that IdentityUpdateTransition predicted fee > real fee', async () => {
      await stateRepository.storeIdentity(identity);

      initialAppHash = await groveDBStore.getRootHash();

      const newPublicKeys = [];
      const disablePublicKeys = [];

      const newPrivateKeys = [];
      for (let i = 0; i < identityUpdateTransitionSchema.properties.addPublicKeys.maxItems; i++) {
        const pk = new PrivateKey();
        newPrivateKeys.push(pk);

        newPublicKeys.push(
          new IdentityPublicKey({
            id: i + identity.getPublicKeys().length,
            type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
            data: pk.toPublicKey().toBuffer(),
            purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
            securityLevel: i === 0
              ? IdentityPublicKey.SECURITY_LEVELS.MASTER : IdentityPublicKey.SECURITY_LEVELS.HIGH,
            readOnly: false,
          }),
        );

        disablePublicKeys.push(identity.getPublicKeyById(i));
      }

      const stateTransition = dpp.identity.createIdentityUpdateTransition(
        identity,
        {
          add: newPublicKeys,
          disable: disablePublicKeys,
        },
      );

      const [signerKey] = identity.getPublicKeys();

      const starterPromise = Promise.resolve(null);

      await stateTransition.getPublicKeysToAdd().reduce(
        (previousPromise, publicKey) => previousPromise.then(async () => {
          const privateKey = newPrivateKeys[publicKey.getId() - identity.getPublicKeys().length];

          if (!privateKey) {
            throw new Error(`Private key for key ${publicKey.getId()} not found`);
          }

          stateTransition.setSignaturePublicKeyId(signerKey.getId());

          await stateTransition.signByPrivateKey(privateKey, publicKey.getType());

          publicKey.setSignature(stateTransition.getSignature());

          stateTransition.setSignature(undefined);
          stateTransition.setSignaturePublicKeyId(undefined);
        }),
        starterPromise,
      );

      await stateTransition.sign(
        identity.getPublicKeyById(0),
        privateKeys[0],
      );

      // validate state transition

      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);

      expect(validateBasicResult.isValid()).to.be.true();

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();
      expect(initialAppHash).to.deep.equal(appHash);

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
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

      await stateTransition.sign(
        identity.getPublicKeyById(1),
        privateKeys[1],
      );

      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);

      expect(validateBasicResult.isValid()).to.be.true();

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();
      expect(initialAppHash).to.deep.equal(appHash);

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
    });

    it('should check that DataContractUpdate predicted fee > real fee', async () => {
      await stateRepository.storeDataContract(dataContract);

      initialAppHash = await groveDBStore.getRootHash();

      dataContract.setVersion(2);
      const stateTransition = dpp.dataContract.createDataContractUpdateTransition(dataContract);

      await stateTransition.sign(
        identity.getPublicKeyById(1),
        privateKeys[1],
      );

      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);

      expect(validateBasicResult.isValid()).to.be.true();

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();
      expect(initialAppHash).to.deep.equal(appHash);

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
    });
  });

  describe('Document', () => {
    let documents;
    let dataContract;

    beforeEach(async () => {
      const dataContractDocuments = createDataContractDocuments();
      dataContract = dpp.dataContract.create(identity.getId(), dataContractDocuments);
      const validationResult = await dpp.dataContract.validate(dataContract);
      expect(validationResult.isValid()).to.be.true();

      documents = [];

      let i = 0;
      for (const documentType of Object.keys(dataContractDocuments)) {
        const data = {};

        for (const propertyName of Object.keys(dataContractDocuments[documentType].properties)) {
          data[propertyName] = new Array(62).fill('d').join('');
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

      await stateTransition.sign(
        identity.getPublicKeyById(1),
        privateKeys[1],
      );

      const executionContext = stateTransition.getExecutionContext();
      await stateRepository.storeDataContract(dataContract, executionContext);

      initialAppHash = await groveDBStore.getRootHash();

      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);

      expect(validateBasicResult.isValid()).to.be.true();

      executionContext.enableDryRun();

      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
    });

    it('should check that DocumentsBatchTransition update predicted fee > real fee', async () => {
      const createStateTransition = dpp.document.createStateTransition({
        create: documents,
      });

      const stateTransition = dpp.document.createStateTransition({
        replace: documents,
      });

      await stateTransition.sign(
        identity.getPublicKeyById(1),
        privateKeys[1],
      );

      const executionContext = stateTransition.getExecutionContext();

      await stateRepository.storeDataContract(dataContract, executionContext);

      await dpp.stateTransition.apply(createStateTransition);

      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);

      expect(validateBasicResult.isValid()).to.be.true();

      executionContext.actualOperations = [];
      executionContext.enableDryRun();

      initialAppHash = await groveDBStore.getRootHash();

      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
    });

    it('should check that DocumentsBatchTransition delete predicted fee > real fee', async () => {
      const createStateTransition = dpp.document.createStateTransition({
        create: documents,
      });

      const stateTransition = dpp.document.createStateTransition({
        delete: documents,
      });

      await stateTransition.sign(
        identity.getPublicKeyById(1),
        privateKeys[1],
      );

      const executionContext = stateTransition.getExecutionContext();
      await stateRepository.storeDataContract(dataContract, executionContext);

      await dpp.stateTransition.apply(createStateTransition);

      initialAppHash = await groveDBStore.getRootHash();

      const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);

      expect(validateBasicResult.isValid()).to.be.true();

      executionContext.actualOperations = [];
      executionContext.enableDryRun();

      const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(validateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();
      const dryRunOperations = executionContext.getOperations();

      executionContext.disableDryRun();
      executionContext.clearDryOperations();

      const appHash = await groveDBStore.getRootHash();

      expect(initialAppHash).to.deep.equal(appHash);

      const actualValidateStateResult = await dpp.stateTransition.validateState(stateTransition);
      expect(actualValidateStateResult.isValid()).to.be.true();

      await dpp.stateTransition.validateSignature(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();
      const actualOperations = executionContext.getOperations();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);

      expectFeeCalculationsAreValid(dryRunOperations, actualOperations);
    });
  });
});
