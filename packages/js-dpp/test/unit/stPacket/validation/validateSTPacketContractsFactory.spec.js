const validateSTPacketContractsFactory = require('../../../../lib/stPacket/validation/validateSTPacketContractsFactory');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe.skip('validateSTPacketContractsFactory', () => {
  let rawSTPacket;
  let rawDataContract;
  let dataContract;
  let validateSTPacketContracts;
  let validateContractMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toJSON();
    rawSTPacket = {
      contractId: dataContract.getId(),
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [rawDataContract],
      documents: [],
    };

    validateContractMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacketContracts = validateSTPacketContractsFactory(
      validateContractMock,
    );
  });

  it('should return invalid result if Contract is wrong', () => {
    const contractError = new ConsensusError('test');

    validateContractMock.returns(
      new ValidationResult([contractError]),
    );

    const result = validateSTPacketContracts(rawSTPacket);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(contractError);
  });

  it('should return valid result if Contract is valid', () => {
    const result = validateSTPacketContracts(rawSTPacket);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
