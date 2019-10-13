const verifyContract = require('../../../../lib/stPacket/verification/verifyContract');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');

const ContractAlreadyPresentError = require('../../../../lib/errors/ContractAlreadyPresentError');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

describe.skip('verifyContract', () => {
  let contract;
  let stPacket;

  beforeEach(() => {
    contract = getContractFixture();

    stPacket = new STPacket(contract.getId());
    stPacket.setContract(contract);
  });

  it('should return invalid result if Contract is already present', async () => {
    const result = await verifyContract(stPacket, contract);

    expectValidationError(result, ContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getContract()).to.equal(contract);
  });

  it('should return valid result if Contract is not present', async () => {
    const result = await verifyContract(stPacket, undefined);

    expectValidationError(result, ContractAlreadyPresentError, 0);
  });
});
