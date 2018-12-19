const validateSTPacketDapContractsFactory = require('../../../../lib/stPacket/validation/validateSTPacketDapContractsFactory');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');

const InvalidSTPacketContractIdError = require('../../../../lib/errors/InvalidSTPacketContractIdError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketDapContractsFactory', () => {
  let rawStPacket;
  let rawDapContract;
  let dapContract;
  let validateSTPacketDapContracts;
  let validateDapContractMock;
  let createDapContractMock;

  beforeEach(function beforeEach() {
    dapContract = getDapContractFixture();
    rawDapContract = dapContract.toJSON();
    rawStPacket = {
      contractId: dapContract.getId(),
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [rawDapContract],
      objects: [],
    };

    createDapContractMock = this.sinonSandbox.stub().returns(dapContract);
    validateDapContractMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacketDapContracts = validateSTPacketDapContractsFactory(
      validateDapContractMock,
      createDapContractMock,
    );
  });

  it('should return invalid result if DAP Contract is wrong', () => {
    const dapContractError = new ConsensusError('test');

    validateDapContractMock.returns(
      new ValidationResult([dapContractError]),
    );

    const result = validateSTPacketDapContracts(rawStPacket.contracts, rawStPacket);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.equal(dapContractError);

    expect(createDapContractMock).to.be.not.called();
  });

  it('should return invalid result if STPacket\'s contractId is not equal to DAP Contract ID', () => {
    rawStPacket.contractId = 'wrong';

    const result = validateSTPacketDapContracts(rawStPacket.contracts, rawStPacket);

    expectValidationError(result, InvalidSTPacketContractIdError);

    expect(createDapContractMock).to.be.calledOnceWith(rawDapContract);

    const [error] = result.getErrors();

    expect(error.getDapContractId()).to.be.equal(rawStPacket.contractId);
    expect(error.getDapContract()).to.be.equal(dapContract);
  });

  it('should return valid result if DAP Contract is valid and STPacket\'s contractId is correct', () => {
    createDapContractMock.returns(dapContract);

    const result = validateSTPacketDapContracts(rawStPacket.contracts, rawStPacket);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(createDapContractMock).to.be.calledOnceWith(rawDapContract);
  });
});
