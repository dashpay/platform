const verifyDapContractFactory = require('../../../../lib/stPacket/verification/verifyDapContractFactory');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

const DapContractAlreadyPresentError = require('../../../../lib/errors/DapContractAlreadyPresentError');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

describe('verifyDapContract', () => {
  let verifyDapContract;
  let dataProviderMock;
  let dapContract;
  let stPacket;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    verifyDapContract = verifyDapContractFactory(dataProviderMock);

    dapContract = getDapContractFixture();

    stPacket = new STPacket(dapContract.getId());
    stPacket.setDapContract(dapContract);
  });

  it('should return invalid result if DAP Contract is already present', async () => {
    dataProviderMock.fetchDapContract.resolves(dapContract);

    const result = await verifyDapContract(stPacket);

    expectValidationError(result, DapContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDapContract()).to.be.equal(dapContract);

    expect(dataProviderMock.fetchDapContract).to.be.calledOnceWith(dapContract.getId());
  });

  it('should return valid result if DAP Contract is not present', async () => {
    const result = await verifyDapContract(stPacket);

    expectValidationError(result, DapContractAlreadyPresentError, 0);
  });
});
