const crypto = require('crypto');

const Long = require('long');

// TODO: should we take it from other place?
const identityUpdateTransitionSchema = require('@dashevo/dpp/schema/identity/stateTransition/identityUpdate.json');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const getInstantAssetLockProofFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getInstantAssetLockProofFixture');

const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');

const getBiggestPossibleIdentity = require('../../../lib/test/mock/getBiggestPossibleIdentity');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const createDataContractDocuments = require('../../../lib/test/fixtures/createDataContractDocuments');
const BlockInfo = require('../../../lib/blockExecution/BlockInfo');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {AbstractStateTransition} stateTransition
 * @return {Promise<void>}
 */
async function validateStateTransition(dpp, stateTransition) {
  const validateBasicResult = await dpp.stateTransition.validateBasic(stateTransition);
  expect(validateBasicResult.isValid()).to.be.true();

  const validateSignatureResult = await dpp.stateTransition.validateSignature(stateTransition);
  expect(validateSignatureResult.isValid()).to.be.true();

  const validateFeeResult = await dpp.stateTransition.validateFee(stateTransition);
  expect(validateFeeResult.isValid()).to.be.true();

  const validateStateResult = await dpp.stateTransition.validateState(stateTransition);
  expect(validateStateResult.isValid()).to.be.true();

  const applyResult = await dpp.stateTransition.validateState(stateTransition);
  expect(applyResult.isValid()).to.be.true();
}

/**
 * @param {Object} dppWasm
 * @returns expectPredictedFeeHigherOrEqualThanActual
 */
function expectPredictedFeeHigherOrEqualThanActualFactory(dppWasm) {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {GroveDBStore} groveDBStore
   * @param {AbstractStateTransition} stateTransition
   * @return {Promise<void>}
   */
  async function expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition) {
    const { StateTransitionExecutionContext } = dppWasm;
    // Execute state transition without dry run

    const actualExecutionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(actualExecutionContext);

    await validateStateTransition(dpp, stateTransition);

    // Execute state transition with dry run enabled

    const predictedExecutionContext = new StateTransitionExecutionContext();

    predictedExecutionContext.enableDryRun();

    stateTransition.setExecutionContext(predictedExecutionContext);

    const initialAppHash = await groveDBStore.getRootHash();

    await validateStateTransition(dpp, stateTransition);

    // AppHash shouldn't be changed after dry run
    const appHashAfterDryRun = await groveDBStore.getRootHash();

    expect(appHashAfterDryRun).to.deep.equal(initialAppHash);

    // Compare operations

    // TODO: Processing fees are disabled for v0.23
    // const actualOperations = actualExecutionContext.getOperations();
    // const predictedOperations = predictedExecutionContext.getOperations();

    // expect(predictedOperations).to.have.lengthOf(actualOperations.length);

    // Compare fees

    // stateTransition.setExecutionContext(actualExecutionContext);
    // const actualFees = calculateStateTransitionFee(stateTransition);
    //
    // stateTransition.setExecutionContext(predictedExecutionContext);
    // const predictedFees = calculateStateTransitionFee(stateTransition);
    //
    // expect(predictedFees).to.be.greaterThanOrEqual(actualFees);
    //
    // predictedOperations.forEach((predictedOperation, i) => {
    //   expect(predictedOperation.getStorageCost()).to.be.greaterThanOrEqual(
    //     actualOperations[i].getStorageCost(),
    //   );
    //
    //   expect(predictedOperation.getProcessingCost()).to.be.greaterThanOrEqual(
    //     actualOperations[i].getProcessingCost(),
    //   );
    // });
  }

  return expectPredictedFeeHigherOrEqualThanActual;
}

describe('feesPrediction', () => {
  let dpp;
  let container;
  let stateRepository;
  let identity;
  let groveDBStore;
  let blockInfo;
  let IdentityPublicKey;
  let Identity;
  let BlsSignatures;
  let expectPredictedFeeHigherOrEqualThanActual;
  let KeyPurpose;
  let KeyType;
  let KeySecurityLevel;

  before(function before() {
    ({
      IdentityPublicKey, Identity, BlsSignatures, KeyPurpose, KeyType, KeySecurityLevel,
    } = this.dppWasm);

    expectPredictedFeeHigherOrEqualThanActual = expectPredictedFeeHigherOrEqualThanActualFactory(
      this.dppWasm,
    );
  });

  beforeEach(async function beforeEach() {
    container = await createTestDIContainer(this.blsAdapter, this.dppWasm);

    const latestBlockExecutionContext = container.resolve('latestBlockExecutionContext');

    blockInfo = new BlockInfo(1, 0, Date.now());

    latestBlockExecutionContext.getEpochInfo = this.sinon.stub().returns({
      currentEpochIndex: blockInfo.epoch,
    });

    latestBlockExecutionContext.getTimeMs = this.sinon.stub().returns(blockInfo.timeMs);
    latestBlockExecutionContext.getHeight = this.sinon.stub().returns(Long.fromNumber(0));

    dpp = container.resolve('dpp');

    stateRepository = container.resolve('stateRepository');
    groveDBStore = container.resolve('groveDBStore');

    /**
     * @type {Drive}
     */
    const rsDrive = container.resolve('rsDrive');
    await rsDrive.createInitialStateStructure();
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  describe('Identity', () => {
    let assetLockPrivateKey;
    let instantAssetLockProof;
    let privateKeys;

    beforeEach(async function beforeEachFunction() {
      assetLockPrivateKey = new PrivateKey();

      instantAssetLockProof = getInstantAssetLockProofFixture(assetLockPrivateKey);

      identity = getBiggestPossibleIdentity(this.dppWasm);
      identity.id = instantAssetLockProof.createIdentifier();
      identity.setAssetLockProof(instantAssetLockProof);

      // Generate real keys
      const { BasicSchemeMPL } = await BlsSignatures.getInstance();

      privateKeys = identity.getPublicKeys().map((identityPublicKey) => {
        const randomBytes = new Uint8Array(crypto.randomBytes(256));

        const privateKey = BasicSchemeMPL.keyGen(randomBytes);
        const publicKey = privateKey.getG1();
        const publicKeyBuffer = Buffer.from(publicKey.serialize());

        identityPublicKey.setData(publicKeyBuffer);

        const result = Buffer.from(privateKey.serialize());

        privateKey.delete();
        publicKey.delete();

        return result;
      });

      stateRepository.verifyInstantLock = this.sinon.stub().resolves(true);
    });

    describe('IdentityCreateTransition', () => {
      it('should have predicted fee more than actual fee', async () => {
        const stateTransition = dpp.identity.createIdentityCreateTransition(identity);

        // Sign public keys
        const publicKeys = stateTransition.getPublicKeys();

        for (let i = 0; i < publicKeys.length; i++) {
          await stateTransition.signByPrivateKey(
            privateKeys[i],
            KeyType.BLS12_381,
          );

          publicKeys[i].setSignature(stateTransition.getSignature());

          stateTransition.setSignature(undefined);
        }

        // Sign state transition
        await stateTransition.signByPrivateKey(
          assetLockPrivateKey,
          KeyType.ECDSA_SECP256K1,
        );

        await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
      });
    });

    describe('IdentityTopUpTransition', () => {
      it('should have predicted fee more than actual fee', async () => {
        await stateRepository.createIdentity(identity);

        const stateTransition = dpp.identity.createIdentityTopUpTransition(
          identity.getId(),
          instantAssetLockProof,
        );

        await stateTransition.signByPrivateKey(
          assetLockPrivateKey,
          KeyType.ECDSA_SECP256K1,
        );

        await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
      });
    });

    describe('IdentityUpdateTransition', () => {
      it('should have predicted fee more than actual fee', async () => {
        await stateRepository.createIdentity(identity);

        const newIdentityPublicKeys = [];
        const disableIdentityPublicKeys = [];

        const { BasicSchemeMPL } = await BlsSignatures.getInstance();

        const newPrivateKeys = [];
        for (let i = 0; i < identityUpdateTransitionSchema.properties.addPublicKeys.maxItems; i++) {
          const randomBytes = new Uint8Array(crypto.randomBytes(256));
          const privateKey = BasicSchemeMPL.keyGen(randomBytes);
          const publicKey = privateKey.getG1();
          const publicKeyBuffer = Buffer.from(publicKey.serialize());

          newPrivateKeys.push(privateKey);

          newIdentityPublicKeys.push(
            new IdentityPublicKey({
              id: i + identity.getPublicKeys().length,
              type: KeyType.BLS12_381,
              data: publicKeyBuffer,
              purpose: KeyPurpose.AUTHENTICATION,
              securityLevel: i === 0
                ? KeySecurityLevel.MASTER : KeySecurityLevel.HIGH,
              readOnly: false,
            }),
          );

          disableIdentityPublicKeys.push(identity.getPublicKeyById(i));

          publicKey.delete();
        }

        const stateTransition = dpp.identity.createIdentityUpdateTransition(
          identity,
          {
            add: newIdentityPublicKeys,
            disable: disableIdentityPublicKeys,
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

        await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
      });
    });
  });

  describe('DataContract', () => {
    let dataContract;
    let privateKey;

    beforeEach(async () => {
      // Create identity

      privateKey = new PrivateKey();

      identity = new Identity({
        protocolVersion: 1,
        id: generateRandomIdentifier().toBuffer(),
        publicKeys: [
          {
            id: 0,
            type: KeyType.BLS12_381,
            purpose: KeyPurpose.AUTHENTICATION,
            securityLevel: KeySecurityLevel.MASTER,
            readOnly: false,
            data: Buffer.alloc(48).fill(255),
          },
          {
            id: 1,
            type: KeyType.ECDSA_SECP256K1,
            purpose: KeyPurpose.AUTHENTICATION,
            securityLevel: KeySecurityLevel.HIGH,
            readOnly: false,
            data: privateKey.toPublicKey().toBuffer(),
          },
        ],
        balance: Math.floor(9223372036854775807 / 10000),
        revision: 0,
      });

      await stateRepository.createIdentity(identity);

      // Generate Data Contract

      const documents = createDataContractDocuments();

      dataContract = dpp.dataContract.create(identity.getId(), documents);
    });

    describe('DataContractCreate', () => {
      it('should have predicted fee more than actual fee', async () => {
        const stateTransition = dpp.dataContract.createDataContractCreateTransition(dataContract);

        await stateTransition.sign(
          identity.getPublicKeyById(1),
          privateKey,
        );

        await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
      });
    });

    describe('DataContractUpdate', () => {
      it('should have predicted fee more than actual fee', async () => {
        await stateRepository.createDataContract(dataContract);

        dataContract.incrementVersion();

        const documents = dataContract.getDocuments();

        documents.newDoc = {
          type: 'object',
          indices: [
            {
              name: 'onwerIdToUser',
              properties: [
                { $ownerId: 'asc' },
                { user: 'asc' },
              ],
              unique: true,
            },
          ],
          properties: {
            user: {
              type: 'string',
              maxLength: 63,
            },
            publicKey: {
              type: 'array',
              byteArray: true,
              maxItems: 33,
            },
          },
          required: ['user', 'publicKey'],
          additionalProperties: false,
        };

        dataContract.setDocuments(documents);

        const stateTransition = dpp.dataContract.createDataContractUpdateTransition(dataContract);

        await stateTransition.sign(
          identity.getPublicKeyById(1),
          privateKey,
        );

        await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
      });
    });
  });

  describe('Document', () => {
    let documents;
    let dataContract;
    let privateKey;

    beforeEach(async () => {
      // Create Identity

      privateKey = new PrivateKey();

      identity = new Identity({
        protocolVersion: 1,
        id: generateRandomIdentifier().toBuffer(),
        publicKeys: [
          {
            id: 0,
            type: KeyType.BLS12_381,
            purpose: KeyPurpose.AUTHENTICATION,
            securityLevel: KeySecurityLevel.MASTER,
            readOnly: false,
            data: Buffer.alloc(48).fill(255),
          },
          {
            id: 1,
            type: KeyType.ECDSA_SECP256K1,
            purpose: KeyPurpose.AUTHENTICATION,
            securityLevel: KeySecurityLevel.HIGH,
            readOnly: false,
            data: privateKey.toPublicKey().toBuffer(),
          },
        ],
        balance: Math.floor(9223372036854775807 / 10000),
        revision: 0,
      });

      await stateRepository.createIdentity(identity);

      // Create Data Contract

      const documentTypes = createDataContractDocuments();

      dataContract = dpp.dataContract.create(identity.getId(), documentTypes);

      await stateRepository.createDataContract(dataContract);

      // Create documents

      documents = [];

      let i = 0;
      for (const documentType of Object.keys(documentTypes)) {
        const data = {};

        for (const propertyName of Object.keys(documentTypes[documentType].properties)) {
          data[propertyName] = `${crypto.randomBytes(31).toString('hex')}a`;
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

    describe('DocumentsBatchTransition', () => {
      context('create', () => {
        it('should have predicted fee more than actual fee', async () => {
          const stateTransition = dpp.document.createStateTransition({
            create: documents,
          });

          await stateTransition.sign(
            identity.getPublicKeyById(1),
            privateKey,
          );

          await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
        });
      });

      context('replace', () => {
        it('should have predicted fee more than actual fee', async () => {
          for (const document of documents) {
            await stateRepository.createDocument(document);
          }

          for (const document of documents) {
            const data = document.getData();

            for (const propertyName of Object.keys(data)) {
              data[propertyName] = `${crypto.randomBytes(31).toString('hex')}b`;
            }

            document.setData(data);
          }

          const stateTransition = dpp.document.createStateTransition({
            replace: documents,
          });

          await stateTransition.sign(
            identity.getPublicKeyById(1),
            privateKey,
          );

          await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
        });
      });

      context('delete', () => {
        it('should have predicted fee more than actual fee', async () => {
          for (const document of documents) {
            await stateRepository.createDocument(document);
          }

          const stateTransition = dpp.document.createStateTransition({
            delete: documents,
          });

          await stateTransition.sign(
            identity.getPublicKeyById(1),
            privateKey,
          );

          await expectPredictedFeeHigherOrEqualThanActual(dpp, groveDBStore, stateTransition);
        });
      });
    });
  });
});
