const verifyDapObjectsFactory = require('../../../../lib/stPacket/verification/verifyDapObjectsFactory');

const STPacket = require('../../../../lib/stPacket/STPacket');
const DapObject = require('../../../../lib/dapObject/DapObject');

const getDapObjectsFixture = require('../../../../lib/test/fixtures/getDapObjectsFixture');
const getDapContractFixture = require('../../../../lib/test/fixtures/getDapContractFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const InvalidDapObjectScopeError = require('../../../../lib/errors/InvalidDapObjectScopeError');
const DapObjectAlreadyPresentError = require('../../../../lib/errors/DapObjectAlreadyPresentError');
const DapObjectNotFoundError = require('../../../../lib/errors/DapObjectNotFoundError');
const InvalidDapObjectRevisionError = require('../../../../lib/errors/InvalidDapObjectRevisionError');
const InvalidDapObjectActionError = require('../../../../lib/stPacket/errors/InvalidDapObjectActionError');

describe('verifyDapObjects', () => {
  let verifyDapObjects;
  let fetchDapObjectsByObjectsMock;
  let stPacket;
  let dapObjects;
  let dapContract;
  let userId;

  beforeEach(function beforeEach() {
    ({ userId } = getDapObjectsFixture);

    dapObjects = getDapObjectsFixture();
    dapContract = getDapContractFixture();

    stPacket = new STPacket(dapContract.getId());
    stPacket.setDapObjects(dapObjects);

    fetchDapObjectsByObjectsMock = this.sinonSandbox.stub();

    verifyDapObjects = verifyDapObjectsFactory(fetchDapObjectsByObjectsMock);
  });

  it('should return invalid result if Dap Object has wrong scope', async () => {
    dapObjects[0].scope = 'wrong';

    fetchDapObjectsByObjectsMock.resolves([]);

    const result = await verifyDapObjects(stPacket, userId);

    expectValidationError(result, InvalidDapObjectScopeError);

    expect(fetchDapObjectsByObjectsMock).to.be.calledOnceWith(
      stPacket.getDapContractId(),
      dapObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDapObject()).to.be.equal(dapObjects[0]);
  });

  it('should return invalid result if Dap Object with action "create" is already present', async () => {
    fetchDapObjectsByObjectsMock.resolves([dapObjects[0]]);

    const result = await verifyDapObjects(stPacket, userId);

    expectValidationError(result, DapObjectAlreadyPresentError);

    expect(fetchDapObjectsByObjectsMock).to.be.calledOnceWith(
      stPacket.getDapContractId(),
      dapObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDapObject()).to.be.equal(dapObjects[0]);
  });

  it('should return invalid result if Dap Object with action "update" is not present', async () => {
    dapObjects[0].setAction(DapObject.ACTIONS.UPDATE);

    fetchDapObjectsByObjectsMock.resolves([]);

    const result = await verifyDapObjects(stPacket, userId);

    expectValidationError(result, DapObjectNotFoundError);

    expect(fetchDapObjectsByObjectsMock).to.be.calledOnceWith(
      stPacket.getDapContractId(),
      dapObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDapObject()).to.be.equal(dapObjects[0]);
  });

  it('should return invalid result if Dap Object with action "delete" is not present', async () => {
    dapObjects[0].setAction(DapObject.ACTIONS.DELETE);

    fetchDapObjectsByObjectsMock.resolves([]);

    const result = await verifyDapObjects(stPacket, userId);

    expectValidationError(result, DapObjectNotFoundError);

    expect(fetchDapObjectsByObjectsMock).to.be.calledOnceWith(
      stPacket.getDapContractId(),
      dapObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDapObject()).to.be.equal(dapObjects[0]);
  });

  it('should return invalid result if Dap Object with action "update" has wrong revision', async () => {
    dapObjects[0].setAction(DapObject.ACTIONS.UPDATE);

    fetchDapObjectsByObjectsMock.resolves([dapObjects[0]]);

    const result = await verifyDapObjects(stPacket, userId);

    expectValidationError(result, InvalidDapObjectRevisionError);

    expect(fetchDapObjectsByObjectsMock).to.be.calledOnceWith(
      stPacket.getDapContractId(),
      dapObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDapObject()).to.be.equal(dapObjects[0]);
  });

  it('should return invalid result if Dap Object with action "delete" has wrong revision', async () => {
    dapObjects[0].setAction(DapObject.ACTIONS.DELETE);

    fetchDapObjectsByObjectsMock.resolves([dapObjects[0]]);

    const result = await verifyDapObjects(stPacket, userId);

    expectValidationError(result, InvalidDapObjectRevisionError);

    expect(fetchDapObjectsByObjectsMock).to.be.calledOnceWith(
      stPacket.getDapContractId(),
      dapObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDapObject()).to.be.equal(dapObjects[0]);
  });

  it('should throw an error if Dap Object has invalid action', async () => {
    dapObjects[0].setAction(5);

    fetchDapObjectsByObjectsMock.resolves([dapObjects[0]]);

    let error;
    try {
      await verifyDapObjects(stPacket, userId);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidDapObjectActionError);
    expect(error.getDapObject()).to.be.equal(dapObjects[0]);
  });

  it('should return valid result if Dap Objects are valid', async () => {
    const fetchedDapObjects = [
      new DapObject(dapObjects[1].toJSON()),
      new DapObject(dapObjects[2].toJSON()),
    ];

    fetchDapObjectsByObjectsMock.resolves(fetchedDapObjects);

    dapObjects[1].setAction(DapObject.ACTIONS.UPDATE);
    dapObjects[1].setRevision(1);

    dapObjects[2].setAction(DapObject.ACTIONS.DELETE);
    dapObjects[2].setRevision(1);

    const result = await verifyDapObjects(stPacket, userId);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
