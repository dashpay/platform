const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const identityCreateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityCreate.json');
const identityTopUpTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityTopUp.json');
const identityUpdateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityUpdate.json');
const dataContractCreateTransitionSchema = require('@dashevo/dpp/schema/dataContract/stateTransition/dataContractCreate.json');
const documentsBatchTransitionSchema = require('@dashevo/dpp/schema/document/stateTransition/documentsBatch.json');
const dataContractMetaSchema = require('@dashevo/dpp/schema/dataContract/dataContractMeta.json');
const DataContractFactory = require('@dashevo/dpp/lib/dataContract/DataContractFactory');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('checkFeePrediction', () => {
  let dpp;
  let container;
  let stateRepository;
  let instantAssetLockProof;
  let identity;

  beforeEach(async function beforeEach() {
    container = await createTestDIContainer();

    const blockExecutionContext = container.resolve('blockExecutionContext');
    blockExecutionContext.getHeader = this.sinon.stub().returns(
      { time: { seconds: 1651585250 } },
    );

    dpp = container.resolve('dpp');

    stateRepository = container.resolve('stateRepository');

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

    identity = getIdentityFixture();
    identity.id = instantAssetLockProof.createIdentifier();
    identity.setAssetLockProof(instantAssetLockProof);
    identity.setBalance(Number.MAX_VALUE);
    identity.setRevision(Number.MAX_VALUE);
    identity.setPublicKeys(publicKeys);
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

      executionContext.disableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });

    it('should check that IdentityTopUpTransition predicted fee > real fee', async () => {
      await stateRepository.storeIdentity(identity);

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

      executionContext.disableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });

    it('should check that IdentityUpdateTransition predicted fee > real fee', async () => {
      await stateRepository.storeIdentity(identity);

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

      const signature = Buffer.alloc(identityTopUpTransitionSchema.properties.signature.maxItems, '1');
      stateTransition.setSignature(signature);
      stateTransition.setSignaturePublicKeyId(Number.MAX_VALUE);

      stateTransition.setPublicKeysDisabledAt(new Date(Number.MAX_VALUE));

      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();

      executionContext.disableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });
  });

  describe('DataContract', () => {
    let dataContract;

    beforeEach(async () => {
      const name = new Array(62).fill('a').join('');
      const description = '1'; // new Array(4294967295).fill('d').join('');

      const properties = {};
      for (let i = 0; i < dataContractMetaSchema.$defs.documentProperties.maxProperties / 10; i++) {
        properties[`${i}${name}`.slice(0, 62)] = {
          type: 'string',
          additionalProperties: false,
          // uniqueItems: false,
          // maxLength: Number.MAX_VALUE,
          // minLength: Number.MAX_VALUE,
          // minimum: Number.MAX_VALUE,
          // maximum: Number.MAX_VALUE,
          // description,
          // $comment: description,
          // pattern: description,
        };
      }

      const documents = {};

      for (let i = 0; i < dataContractMetaSchema.properties.documents.maxProperties / 10; i++) {
        documents[`${i}${name}`.slice(0, 62)] = {
          type: 'object',
          properties,
          // required: Object.keys(properties),
          additionalProperties: false,
        };
      }

      const factory = new DataContractFactory(createDPPMock(), () => {});
      dataContract = factory.create(identity.getId(), documents);
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

      executionContext.disableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });

    it('should check that DataContractUpdate predicted fee > real fee', async () => {
      const stateTransition = dpp.dataContract.createDataContractUpdateTransition(dataContract);
      const executionContext = stateTransition.getExecutionContext();

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();

      executionContext.disableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });
  });

  describe('Document', () => {
    let documents;
    let dataContract;

    beforeEach(async () => {
      dataContract = getDataContractFixture(identity.getId());
      documents = getDocumentsFixture(dataContract);
    });

    it('should check that DocumentsBatchTransition create predicted fee > real fee', async () => {
      const stateTransition = dpp.document.createStateTransition({
        create: documents,
      });

      stateTransition.setSignaturePublicKeyId(Number.MAX_VALUE);
      const signature = Buffer.alloc(documentsBatchTransitionSchema.properties.signature.maxItems, '1');
      stateTransition.setSignature(signature);

      //
      // const executionContext = stateTransition.getExecutionContext();
      // await stateRepository.storeDataContract(dataContract, executionContext);
      //
      // executionContext.enableDryRun();
      //
      // await dpp.stateTransition.validateState(stateTransition);
      // await dpp.stateTransition.apply(stateTransition);
      // const predictedStateTransitionFee = stateTransition.calculateFee();
      //
      // executionContext.disableDryRun();
      //
      // await dpp.stateTransition.validateState(stateTransition);
      // await dpp.stateTransition.apply(stateTransition);
      // const realStateTransitionFee = stateTransition.calculateFee();
      //
      // expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });

    it('should check that DocumentsBatchTransition update predicted fee > real fee', async () => {
      // const documentToReplace = documents[0];
      // documentToReplace.setData({ name: 'newName' });

      const stateTransition = dpp.document.createStateTransition({
        replace: documents,
      });

      const executionContext = stateTransition.getExecutionContext();
      await stateRepository.storeDataContract(dataContract, executionContext);

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();

      executionContext.disableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });

    it('should check that DocumentsBatchTransition create predicted fee > real fee', async () => {
      const stateTransition = dpp.document.createStateTransition({
        delete: documents,
      });

      const executionContext = stateTransition.getExecutionContext();
      await stateRepository.storeDataContract(dataContract, executionContext);

      executionContext.enableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const predictedStateTransitionFee = stateTransition.calculateFee();

      executionContext.disableDryRun();

      await dpp.stateTransition.validateState(stateTransition);
      await dpp.stateTransition.apply(stateTransition);
      const realStateTransitionFee = stateTransition.calculateFee();

      expect(predictedStateTransitionFee).to.be.greaterThan(realStateTransitionFee);
    });
  });
});
