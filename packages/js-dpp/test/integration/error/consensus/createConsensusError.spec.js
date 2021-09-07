const InvalidDataContractIdError = require('../../../../lib/errors/consensus/basic/dataContract/InvalidDataContractIdError');
const generateRandomIdentifier = require('../../../../lib/test/utils/generateRandomIdentifier');
const createConsensusError = require('../../../../lib/errors/consensus/createConsensusError');

describe('createConsensusError', () => {
  it('should create an error instance from code and arguments', () => {
    const expectedId = generateRandomIdentifier();
    const invalidId = Buffer.alloc(16).fill(1);

    const error = new InvalidDataContractIdError(expectedId.toBuffer(), invalidId);

    const restoredError = createConsensusError(error.getCode(), error.getConstructorArguments());

    // Stack will be always different so we need to skip it for comparison
    expect(restoredError.message).to.equal(error.message);
    expect(restoredError.getExpectedId()).to.deep.equal(error.getExpectedId());
    expect(restoredError.getInvalidId()).to.deep.equal(error.getInvalidId());
    expect(restoredError.getConstructorArguments()).to.deep.equal(error.getConstructorArguments());
    expect(restoredError.getCode()).to.deep.equal(error.getCode());
  });
});
