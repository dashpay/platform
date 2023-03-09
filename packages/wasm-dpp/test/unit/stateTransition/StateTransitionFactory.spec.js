const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../dist');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

describe('StateTransitionFactory', function main() {
  this.timeout(100000);

  let factory;
  let stateTransition;
  let rawStateTransition;
  let stateRepositoryMock;
  let StateTransitionFactory;
  let DataContractCreateTransition;

  before(async () => {
    ({
      DataContractCreateTransition, StateTransitionFactory,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const dataContract = getDataContractFixture();

    const rawDataContract = dataContract.toObject();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: rawDataContract,
      entropy: dataContract.getEntropy(),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });
    rawStateTransition = stateTransition.toObject();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.fetchDataContract.resolves(dataContract);
    stateRepositoryMock.createDataContract.resolves();
    stateRepositoryMock.updateDataContract.resolves();
    stateRepositoryMock.fetchDocuments.resolves();
    stateRepositoryMock.createDocument.resolves();
    stateRepositoryMock.updateDocument.resolves();
    stateRepositoryMock.removeDocument.resolves();
    stateRepositoryMock.fetchTransaction.resolves();
    stateRepositoryMock.fetchIdentity.resolves();
    stateRepositoryMock.createIdentity.resolves();
    stateRepositoryMock.addKeysToIdentity.resolves();
    stateRepositoryMock.disableIdentityKeys.resolves();
    stateRepositoryMock.updateIdentityRevision.resolves();
    stateRepositoryMock.addToIdentityBalance.resolves();
    stateRepositoryMock.fetchIdentityBalance.resolves();
    stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves();
    stateRepositoryMock.addToSystemCredits.resolves();
    stateRepositoryMock.fetchLatestPlatformBlockHeight.resolves();
    stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.resolves();
    stateRepositoryMock.verifyInstantLock.resolves();
    stateRepositoryMock.markAssetLockTransactionOutPointAsUsed.resolves();
    stateRepositoryMock.verifyChainLockHeight.resolves();
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.resolves();
    stateRepositoryMock.fetchSMLStore.resolves();
    stateRepositoryMock.fetchLatestWithdrawalTransactionIndex.resolves();
    stateRepositoryMock.enqueueWithdrawalTransaction.resolves();
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves();

    const blsAdapter = await getBlsAdapterMock();

    factory = new StateTransitionFactory(
      stateRepositoryMock,
      blsAdapter,
    );
  });

  describe('createFromObject', () => {
    it('should return new State Transition with data from passed object', async () => {
      const result = await factory.createFromObject(rawStateTransition);

      expect(result.toObject()).to.deep.equal(stateTransition.toObject());
    });

    it('should return new State Transition without validation if "skipValidation" option is passed', async () => {
      const result = await factory.createFromObject(rawStateTransition, { skipValidation: true });

      expect(result.toObject()).to.deep.equal(stateTransition.toObject());
    });
  });

  describe('createFromBuffer', () => {
    it('should return new State Transition from serialized contract', async () => {
      const result = await factory.createFromBuffer(stateTransition.toBuffer());

      expect(result.toObject()).to.deep.equal(stateTransition.toObject());
    });
  });
});
