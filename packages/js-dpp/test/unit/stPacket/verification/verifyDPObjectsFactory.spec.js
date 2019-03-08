const verifyDPObjectsFactory = require('../../../../lib/stPacket/verification/verifyDPObjectsFactory');

const STPacket = require('../../../../lib/stPacket/STPacket');
const DPObject = require('../../../../lib/object/DPObject');

const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');
const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const InvalidDPObjectScopeError = require('../../../../lib/errors/InvalidDPObjectScopeError');
const DPObjectAlreadyPresentError = require('../../../../lib/errors/DPObjectAlreadyPresentError');
const DPObjectNotFoundError = require('../../../../lib/errors/DPObjectNotFoundError');
const InvalidDPObjectRevisionError = require('../../../../lib/errors/InvalidDPObjectRevisionError');
const InvalidDPObjectActionError = require('../../../../lib/stPacket/errors/InvalidDPObjectActionError');

describe('verifyDPObjects', () => {
  let verifyDPObjects;
  let fetchDPObjectsByObjectsMock;
  let stPacket;
  let dpObjects;
  let dpContract;
  let userId;

  beforeEach(function beforeEach() {
    ({ userId } = getDPObjectsFixture);

    dpObjects = getDPObjectsFixture();
    dpContract = getDPContractFixture();

    stPacket = new STPacket(dpContract.getId());
    stPacket.setDPObjects(dpObjects);

    fetchDPObjectsByObjectsMock = this.sinonSandbox.stub();

    verifyDPObjects = verifyDPObjectsFactory(fetchDPObjectsByObjectsMock);
  });

  it('should return invalid result if DPObject has wrong scope', async () => {
    dpObjects[0].scope = 'wrong';

    fetchDPObjectsByObjectsMock.resolves([]);

    const result = await verifyDPObjects(stPacket, userId);

    expectValidationError(result, InvalidDPObjectScopeError);

    expect(fetchDPObjectsByObjectsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      dpObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDPObject()).to.equal(dpObjects[0]);
  });

  it('should return invalid result if DPObject with action "create" is already present', async () => {
    fetchDPObjectsByObjectsMock.resolves([dpObjects[0]]);

    const result = await verifyDPObjects(stPacket, userId);

    expectValidationError(result, DPObjectAlreadyPresentError);

    expect(fetchDPObjectsByObjectsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      dpObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDPObject()).to.equal(dpObjects[0]);
    expect(error.getFetchedDPObject()).to.equal(dpObjects[0]);
  });

  it('should return invalid result if DPObject with action "update" is not present', async () => {
    dpObjects[0].setAction(DPObject.ACTIONS.UPDATE);

    fetchDPObjectsByObjectsMock.resolves([]);

    const result = await verifyDPObjects(stPacket, userId);

    expectValidationError(result, DPObjectNotFoundError);

    expect(fetchDPObjectsByObjectsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      dpObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDPObject()).to.equal(dpObjects[0]);
  });

  it('should return invalid result if DPObject with action "delete" is not present', async () => {
    dpObjects[0].setData({});
    dpObjects[0].setAction(DPObject.ACTIONS.DELETE);

    fetchDPObjectsByObjectsMock.resolves([]);

    const result = await verifyDPObjects(stPacket, userId);

    expectValidationError(result, DPObjectNotFoundError);

    expect(fetchDPObjectsByObjectsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      dpObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDPObject()).to.equal(dpObjects[0]);
  });

  it('should return invalid result if DPObject with action "update" has wrong revision', async () => {
    dpObjects[0].setAction(DPObject.ACTIONS.UPDATE);

    fetchDPObjectsByObjectsMock.resolves([dpObjects[0]]);

    const result = await verifyDPObjects(stPacket, userId);

    expectValidationError(result, InvalidDPObjectRevisionError);

    expect(fetchDPObjectsByObjectsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      dpObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDPObject()).to.equal(dpObjects[0]);
    expect(error.getFetchedDPObject()).to.equal(dpObjects[0]);
  });

  it('should return invalid result if DPObject with action "delete" has wrong revision', async () => {
    dpObjects[0].setData({});
    dpObjects[0].setAction(DPObject.ACTIONS.DELETE);

    fetchDPObjectsByObjectsMock.resolves([dpObjects[0]]);

    const result = await verifyDPObjects(stPacket, userId);

    expectValidationError(result, InvalidDPObjectRevisionError);

    expect(fetchDPObjectsByObjectsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      dpObjects,
    );

    const [error] = result.getErrors();

    expect(error.getDPObject()).to.equal(dpObjects[0]);
  });

  it('should throw an error if DPObject has invalid action', async () => {
    dpObjects[0].setAction(5);

    fetchDPObjectsByObjectsMock.resolves([dpObjects[0]]);

    let error;
    try {
      await verifyDPObjects(stPacket, userId);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(InvalidDPObjectActionError);
    expect(error.getDPObject()).to.equal(dpObjects[0]);
  });

  it('should return valid result if DPObjects are valid', async () => {
    const fetchedDPObjects = [
      new DPObject(dpObjects[1].toJSON()),
      new DPObject(dpObjects[2].toJSON()),
    ];

    fetchDPObjectsByObjectsMock.resolves(fetchedDPObjects);

    dpObjects[1].setAction(DPObject.ACTIONS.UPDATE);
    dpObjects[1].setRevision(1);

    dpObjects[2].setData({});
    dpObjects[2].setAction(DPObject.ACTIONS.DELETE);
    dpObjects[2].setRevision(1);

    const result = await verifyDPObjects(stPacket, userId);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
