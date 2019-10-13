const { Transaction } = require('@dashevo/dashcore-lib');

const DataTriggerExecutionContext = require('../../../lib/dataTrigger/DataTriggerExecutionContext');
const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');
const getDpnsContractFixture = require('../../../lib/test/fixtures/getDpnsContractFixture');

describe('DataTriggerExecutionContext', () => {
  let dataContractMock;
  let dataProviderMock;
  let stateTransitionHeaderMock;

  beforeEach(function beforeEach() {
    dataContractMock = getDpnsContractFixture();
    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    stateTransitionHeaderMock = new Transaction();
  });

  it('should have all getters working', () => {
    const userId = 'user_id';
    const context = new DataTriggerExecutionContext(
      dataProviderMock, userId, dataContractMock, stateTransitionHeaderMock,
    );

    expect(context.getDataContract()).to.be.deep.equal(dataContractMock);
    expect(context.getDataProvider()).to.be.deep.equal(dataProviderMock);
    expect(context.getUserId()).to.be.deep.equal(userId);
    expect(context.getStateTransitionHeader()).to.be.deep.equal(stateTransitionHeaderMock);
  });
});
