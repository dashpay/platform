const bs58 = require('bs58');
const DataTriggerExecutionContext = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionContext');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDpnsContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDpnsContractFixture');
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

describe('DataTriggerExecutionContext', () => {
  let dataContractMock;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    dataContractMock = getDpnsContractFixture();
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
  });

  it('should have all getters working', () => {
    const ownerId = bs58.decode('5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq');
    const context = new DataTriggerExecutionContext(
      stateRepositoryMock,
      ownerId,
      dataContractMock,
      new StateTransitionExecutionContext(),
    );

    expect(context.getDataContract()).to.be.deep.equal(dataContractMock);
    expect(context.getStateRepository()).to.be.deep.equal(stateRepositoryMock);
    expect(context.getOwnerId()).to.be.deep.equal(ownerId);
  });
});
