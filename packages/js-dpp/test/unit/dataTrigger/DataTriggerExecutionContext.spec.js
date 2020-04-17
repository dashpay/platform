const DataTriggerExecutionContext = require('../../../lib/dataTrigger/DataTriggerExecutionContext');
const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');
const getDpnsContractFixture = require('../../../lib/test/fixtures/getDpnsContractFixture');

describe('DataTriggerExecutionContext', () => {
  let dataContractMock;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    dataContractMock = getDpnsContractFixture();
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
  });

  it('should have all getters working', () => {
    const ownerId = 'owner_id';
    const context = new DataTriggerExecutionContext(
      stateRepositoryMock, ownerId, dataContractMock,
    );

    expect(context.getDataContract()).to.be.deep.equal(dataContractMock);
    expect(context.getStateRepository()).to.be.deep.equal(stateRepositoryMock);
    expect(context.getOwnerId()).to.be.deep.equal(ownerId);
  });
});
