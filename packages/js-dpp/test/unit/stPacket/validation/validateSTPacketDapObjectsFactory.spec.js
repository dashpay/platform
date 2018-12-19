const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');
const getDapObjectsFixture = require('../../../../lib/test/fixtures/getDapObjectsFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const validateSTPacketDapObjectsFactory = require('../../../../lib/stPacket/validation/validateSTPacketDapObjectsFactory');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const DuplicatedDapObjectsError = require('../../../../lib/errors/DuplicatedDapObjectsError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketDapObjectsFactory', () => {
  let rawStPacket;
  let dapContract;
  let rawDapObjects;
  let findDuplicatedDapObjectsMock;
  let validateDapObjectMock;
  let validateSTPacketDapObjects;

  beforeEach(function beforeEach() {
    dapContract = getDapContractFixture();
    rawDapObjects = getDapObjectsFixture().map(o => o.toJSON());
    rawStPacket = {
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [],
      objects: rawDapObjects,
    };

    findDuplicatedDapObjectsMock = this.sinonSandbox.stub().returns([]);
    validateDapObjectMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacketDapObjects = validateSTPacketDapObjectsFactory(
      validateDapObjectMock,
      findDuplicatedDapObjectsMock,
    );
  });

  it('should return invalid result if there are duplicates DAP Objects', () => {
    findDuplicatedDapObjectsMock.returns([rawDapObjects[0]]);

    const result = validateSTPacketDapObjects(rawStPacket.objects, dapContract);

    expectValidationError(result, DuplicatedDapObjectsError);

    expect(findDuplicatedDapObjectsMock).to.be.calledOnceWith(rawDapObjects);

    const [error] = result.getErrors();

    expect(error.getDuplicatedDapObject()).to.be.deep.equal([rawDapObjects[0]]);

    expect(validateDapObjectMock).to.be.calledThrice();

    rawStPacket.objects.forEach((rawDapObject) => {
      expect(validateDapObjectMock).to.be.calledWith(rawDapObject, dapContract);
    });
  });

  it('should return invalid result if DAP Objects are invalid', () => {
    const dapObjectError = new ConsensusError('test');

    validateDapObjectMock.onCall(0).returns(
      new ValidationResult([dapObjectError]),
    );

    const result = validateSTPacketDapObjects(rawStPacket.objects, dapContract);

    expectValidationError(result, ConsensusError, 1);

    expect(findDuplicatedDapObjectsMock).to.be.calledOnceWith(rawDapObjects);

    expect(validateDapObjectMock).to.be.calledThrice();

    const [error] = result.getErrors();

    expect(error).to.be.equal(dapObjectError);
  });

  it('should return valid result if there are no duplicate DAP Objects and they are valid', () => {
    const result = validateSTPacketDapObjects(rawStPacket.objects, dapContract);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(findDuplicatedDapObjectsMock).to.be.calledOnceWith(rawDapObjects);

    expect(validateDapObjectMock).to.be.calledThrice();
  });
});
