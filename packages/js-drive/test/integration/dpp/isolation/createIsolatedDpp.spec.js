const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getIdentityCreateTransitionFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture',
);
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');
const DocumentsBatchTransition = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition');
const DataContractCreateTransition = require('@dashevo/dpp/lib/dataContract/stateTransition/DataContractCreateTransition');

const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const { PrivateKey } = require('@dashevo/dashcore-lib');

const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');

const createIsolatedValidatorSnapshot = require('../../../../lib/dpp/isolation/createIsolatedValidatorSnapshot');
const createIsolatedDppFactory = require('../../../../lib/dpp/isolation/createIsolatedDppFactory');

describe('createIsolatedDpp', () => {
  let dataContract;
  let document;
  let identityCreateTransition;
  let identity;
  let documentsBatchTransition;
  let dataContractCreateTransition;

  let stateRepositoryMock;
  let createIsolatedDpp;
  let isolatedValidatorSnapshot;

  before(async () => {
    isolatedValidatorSnapshot = await createIsolatedValidatorSnapshot();
  });

  beforeEach(async function createFixturesAndMocks() {
    const privateKey = new PrivateKey();
    const publicKey = privateKey.toPublicKey().toBuffer();
    const publicKeyId = 1;

    const identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey);

    dataContract = getDataContractFixture();
    const documents = getDocumentsFixture(dataContract);
    [document] = documents;
    document.contractId = dataContract.getId();
    identity = getIdentityFixture();
    identity.type = 2;
    identity.publicKeys = [
      identityPublicKey,
    ];

    identityCreateTransition = getIdentityCreateTransitionFixture();

    const documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });
    documentsBatchTransition = new DocumentsBatchTransition({
      ownerId: getDocumentsFixture.ownerId,
      contractId: dataContract.getId(),
      protocolVersion: 0,
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);
    documentsBatchTransition.sign(identityPublicKey, privateKey);

    dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
      protocolVersion: 0,
    });
    dataContractCreateTransition.sign(identityPublicKey, privateKey);

    identityCreateTransition.publicKeys = [new IdentityPublicKey({
      id: 1,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: privateKey.toPublicKey().toBuffer(),
      isEnabled: true,
    })];
    identityCreateTransition.signByPrivateKey(privateKey);

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    createIsolatedDpp = createIsolatedDppFactory(
      isolatedValidatorSnapshot,
      { memoryLimit: 10, timeout: 300 },
      stateRepositoryMock,
    );
  });

  describe('stateTransition', () => {
    describe('#createFromBuffer', () => {
      describe('DocumentsBatchTransition', () => {
        it('should pass through validation result', async () => {
          delete documentsBatchTransition.signature;

          const serializedDocumentsBatchTransition = documentsBatchTransition.toBuffer();

          const isolatedDpp = await createIsolatedDpp();

          try {
            await isolatedDpp.stateTransition.createFromBuffer(
              serializedDocumentsBatchTransition,
            );

            expect.fail('Error was not thrown');
          } catch (e) {
            expect(e).to.be.an.instanceOf(InvalidStateTransitionError);

            const [error] = e.getErrors();
            expect(error.name).to.equal('JsonSchemaError');
            expect(error.params.missingProperty).to.equal('signature');
          } finally {
            isolatedDpp.dispose();
          }
        });

        it('should create state transition from serialized data', async () => {
          const serializedDocumentsBatchTransition = documentsBatchTransition.toBuffer();

          const isolatedDpp = await createIsolatedDpp();

          try {
            const result = await isolatedDpp.stateTransition.createFromBuffer(
              serializedDocumentsBatchTransition,
            );

            expect(result.toObject()).to.deep.equal(documentsBatchTransition.toObject());
          } finally {
            isolatedDpp.dispose();
          }
        });
      });

      describe('DataContractCreateTransition', () => {
        it('should pass through validation result', async () => {
          delete dataContractCreateTransition.signature;

          const serializedStateTransition = dataContractCreateTransition.toBuffer();

          const isolatedDpp = await createIsolatedDpp();

          try {
            await isolatedDpp.stateTransition.createFromBuffer(
              serializedStateTransition,
            );

            expect.fail('Error was not thrown');
          } catch (e) {
            expect(e).to.be.an.instanceOf(InvalidStateTransitionError);

            const [error] = e.getErrors();
            expect(error.name).to.equal('JsonSchemaError');
            expect(error.params.missingProperty).to.equal('signature');
          } finally {
            isolatedDpp.dispose();
          }
        });

        it('should create state transition from serialized data', async () => {
          const serializedStateTransition = dataContractCreateTransition.toBuffer();

          const isolatedDpp = await createIsolatedDpp();

          try {
            const result = await isolatedDpp.stateTransition.createFromBuffer(
              serializedStateTransition,
            );

            expect(result.toObject()).to.deep.equal(dataContractCreateTransition.toObject());
          } finally {
            isolatedDpp.dispose();
          }
        });
      });

      describe('IdentityCreateTransition', () => {
        it('should pass through validation result', async () => {
          delete identityCreateTransition.protocolVersion;

          const isolatedDpp = await createIsolatedDpp();

          try {
            await isolatedDpp.stateTransition.createFromBuffer(
              identityCreateTransition.toBuffer(),
            );
            expect.fail('Error was not thrown');
          } catch (e) {
            expect(e).to.be.an.instanceOf(InvalidStateTransitionError);

            const [error] = e.getErrors();
            expect(error.name).to.equal('JsonSchemaError');
            expect(error.params.missingProperty).to.equal('protocolVersion');
          } finally {
            isolatedDpp.dispose();
          }
        });

        it('should create state transition from serialized data', async () => {
          const isolatedDpp = await createIsolatedDpp();

          try {
            const result = await isolatedDpp.stateTransition.createFromBuffer(
              identityCreateTransition.toBuffer(),
            );

            expect(result.toObject()).to.deep.equal(identityCreateTransition.toObject());
          } finally {
            isolatedDpp.dispose();
          }
        });
      });
    });
  });
});
