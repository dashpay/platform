const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const getMasternodeRewardSharesDocumentsFixture = require('../../../../lib/test/fixtures/getMasternodeRewardSharesDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const createRewardShareDataTrigger = require('../../../../lib/dataTrigger/rewardShareDataTriggers/createMasternodeRewardSharesDataTrigger');
const DataTriggerExecutionResult = require('../../../../lib/dataTrigger/DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../../../lib/errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');

describe('createMasternodeRewardSharesDataTrigger', () => {
  let contextMock;
  let stateRepositoryMock;
  let documentTransition;
  let topLevelIdentityId;
  let smlStoreMock;
  let smlMock;
  let documentsFixture;

  beforeEach(function beforeEach() {
    topLevelIdentityId = Buffer.from('c286807d463b06c7aba3b9a60acf64c1fc03da8c1422005cd9b4293f08cf0562', 'hex');

    smlMock = {
      getQuorum: this.sinonSandbox.stub(),
      toSimplifiedMNListDiff: this.sinonSandbox.stub(),
      getQuorumsOfType: this.sinonSandbox.stub(),
      getValidMasternodesList: this.sinonSandbox.stub().returns([
        new SimplifiedMNListEntry({
          proRegTxHash: topLevelIdentityId.toString('hex'),
          confirmedHash: '4eb56228c535db3b234907113fd41d57bcc7cdcb8e0e00e57590af27ee88c119',
          service: '192.168.65.2:20101',
          pubKeyOperator: '809519c5f6f3be1c08782ac42ae9a83b6c7205eba43f9a96a4f032ec7a73f1a7c25fa78cce0d6d9c135f7e2c28527179',
          votingAddress: 'yXmprXYP51uzfMyndtWwxz96MnkCKkFc9x',
          isValid: true,
        }),
        new SimplifiedMNListEntry({
          proRegTxHash: 'a3e1edc6bd352eeaf0ae58e30781ef4b127854241a3fe7fddf36d5b7e1dc2b3f',
          confirmedHash: '27a0b637b56af038c45e2fd1f06c2401c8dadfa28ca5e0d19ca836cc984a8378',
          service: '192.168.65.2:20201',
          pubKeyOperator: '987a4873caba62cd45a2f7d4aa6d94519ee6753e9bef777c927cb94ade768a542b0ff34a93231d3a92b4e75ffdaa366e',
          votingAddress: 'ycL7L4mhYoaZdm9TH85svvpfeKtdfo249u',
          isValid: true,
        }),
      ]),
    };

    documentsFixture = getMasternodeRewardSharesDocumentsFixture();

    smlStoreMock = {
      getSMLbyHeight: this.sinonSandbox.stub().returns(smlMock),
      getCurrentSML: this.sinonSandbox.stub().returns(smlMock),
    };

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchSMLStore.resolves(smlStoreMock);
    stateRepositoryMock.fetchIdentity.resolves(null);
    stateRepositoryMock.fetchDocuments.resolves(documentsFixture);

    const [document] = getMasternodeRewardSharesDocumentsFixture();

    [documentTransition] = getDocumentTransitionsFixture({
      create: [document],
    });

    contextMock = {
      getStateRepository: () => stateRepositoryMock,
      getOwnerId: this.sinonSandbox.stub(),
      getDataContract: () => getMasternodeRewardSharesDocumentsFixture.dataContract,
    };
    contextMock.getOwnerId.returns(topLevelIdentityId);
  });

  it('should return an error if percentage > 10000', async () => {
    documentTransition.data.percentage = 9501;

    const result = await createRewardShareDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('Percentage can not be more than 10000');

    expect(stateRepositoryMock.fetchSMLStore).to.be.calledOnce();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      documentTransition.data.payToId,
    );
  });

  it('should return an error if payToId already exists', async () => {
    stateRepositoryMock.fetchIdentity.resolves(topLevelIdentityId);

    const result = await createRewardShareDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal(`Identity ${documentTransition.data.payToId.toString()} already exists`);

    expect(stateRepositoryMock.fetchSMLStore).to.be.calledOnce();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      documentTransition.data.payToId,
    );
  });

  it('should return an error if ownerId is not in SML', async () => {
    contextMock.getOwnerId.returns(getIdentityFixture().getId());

    const result = await createRewardShareDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal(`Owner ID ${getIdentityFixture().getId()} is not in SML`);

    expect(stateRepositoryMock.fetchSMLStore).to.be.calledOnce();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      documentTransition.data.payToId,
    );
  });

  it('should pass', async () => {
    const result = await createRewardShareDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.true();

    expect(stateRepositoryMock.fetchSMLStore).to.be.calledOnce();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      documentTransition.data.payToId,
    );
  });
});
