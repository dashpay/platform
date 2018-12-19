const verifyDapContractFactory = require('../../../../lib/stPacket/verification/verifyDapContractFactory');

const AbstractDataProvider = require('../../../../lib/dataProvider/AbstractDataProvider');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');

const DapContractAlreadyPresentError = require('../../../../lib/errors/DapContractAlreadyPresentError');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

describe('verifyDapContract', () => {
  let verifyDapContract;
  let fetchDapContractMock;
  let dapContract;
  let stPacket;

  beforeEach(function beforeEach() {
    const dataProviderMock = this.sinonSandbox.createStubInstance(AbstractDataProvider, {
      fetchDapContract: this.sinonSandbox.stub(),
    });

    fetchDapContractMock = dataProviderMock.fetchDapContract;

    verifyDapContract = verifyDapContractFactory(dataProviderMock);

    dapContract = getDapContractFixture();

    stPacket = new STPacket(dapContract.getId());
    stPacket.setDapContract(dapContract);
  });

  it('should return invalid result if DAP Contract is already present', async () => {
    fetchDapContractMock.resolves(dapContract);

    const result = await verifyDapContract(stPacket);

    expectValidationError(result, DapContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDapContract()).to.be.equal(dapContract);

    expect(fetchDapContractMock).to.be.calledOnceWith(dapContract.getId());
  });

  it('should return valid result if DAP Contract is not present', async () => {
    const result = await verifyDapContract(stPacket);

    expectValidationError(result, DapContractAlreadyPresentError, 0);
  });
});
