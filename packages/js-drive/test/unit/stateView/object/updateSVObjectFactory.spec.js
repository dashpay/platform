const DPObject = require('@dashevo/dpp/lib/object/DPObject');
const SVObject = require('../../../../lib/stateView/object/SVObject');

const Revision = require('../../../../lib/stateView/revisions/Revision');

const updateSVObjectFactory = require('../../../../lib/stateView/object/updateSVObjectFactory');

const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');
const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');
const getSVObjectsFixture = require('../../../../lib/test/fixtures/getSVObjectsFixture');

describe('updateSVObjectFactory', () => {
  let svObjectRepository;
  let updateSVObject;
  let reference;
  let dpObject;
  let userId;
  let contractId;

  beforeEach(function beforeEach() {
    svObjectRepository = {
      find: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    contractId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    ({ userId } = getDPObjectsFixture);
    [dpObject] = getDPObjectsFixture();

    const createSVObjectRepository = () => svObjectRepository;

    updateSVObject = updateSVObjectFactory(createSVObjectRepository);

    reference = getReferenceFixture();
  });

  it('should store SVObject if action is "create"', async () => {
    await updateSVObject(contractId, userId, reference, dpObject);

    expect(svObjectRepository.store).to.calledOnce();

    const svObject = svObjectRepository.store.getCall(0).args[0];

    expect(svObject).to.be.instanceOf(SVObject);
    expect(svObject.getUserId()).to.be.equal(userId);
    expect(svObject.getDPObject()).to.be.equal(dpObject);
    expect(svObject.getReference()).to.be.equal(reference);
    expect(svObject.getPreviousRevisions()).to.be.deep.equal([]);
    expect(svObject.isDeleted()).to.be.false();
  });

  it('should store SVObject if action is "update" and has previous version', async () => {
    const [previousSVObject] = getSVObjectsFixture();

    svObjectRepository.find.returns(previousSVObject);

    dpObject.setRevision(1);
    dpObject.setAction(DPObject.ACTIONS.UPDATE);

    await updateSVObject(contractId, userId, reference, dpObject);

    expect(svObjectRepository.find).to.be.calledOnceWith(dpObject.getId());
    expect(svObjectRepository.store).to.calledOnce();

    const svObject = svObjectRepository.store.getCall(0).args[0];

    expect(svObject).to.be.instanceOf(SVObject);
    expect(svObject.getUserId()).to.be.equal(userId);
    expect(svObject.getDPObject()).to.be.equal(dpObject);
    expect(svObject.getReference()).to.be.equal(reference);
    expect(svObject.getPreviousRevisions()).to.be.deep.equal([
      previousSVObject.getCurrentRevision(),
    ]);
    expect(svObject.isDeleted()).to.be.false();
  });

  it('should throw error if action is "update" and there is no previous version', async () => {
    svObjectRepository.find.returns(null);

    dpObject.setAction(DPObject.ACTIONS.UPDATE);

    let error;
    try {
      await updateSVObject(contractId, userId, reference, dpObject);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(Error);

    expect(svObjectRepository.find).to.be.calledOnceWith(dpObject.getId());
    expect(svObjectRepository.store).not.to.be.called();
  });

  it('should store SVObject remove ahead versions if action is "update" upon reverting', async () => {
    const previousRevisions = [
      new Revision(0, reference),
      new Revision(1, reference),
      new Revision(2, reference),
      new Revision(3, reference),
    ];

    const isDeleted = false;

    const previousSVObject = new SVObject(
      userId,
      dpObject,
      reference,
      isDeleted,
      previousRevisions,
    );

    svObjectRepository.find.returns(previousSVObject);

    dpObject.setAction(DPObject.ACTIONS.UPDATE);
    dpObject.setRevision(2);

    await updateSVObject(contractId, userId, reference, dpObject, true);

    expect(svObjectRepository.find).to.be.calledOnceWith(dpObject.getId());
    expect(svObjectRepository.store).to.calledOnce();

    const svObject = svObjectRepository.store.getCall(0).args[0];

    expect(svObject).to.be.instanceOf(SVObject);
    expect(svObject.getUserId()).to.be.equal(userId);
    expect(svObject.getDPObject()).to.be.equal(dpObject);
    expect(svObject.getReference()).to.be.equal(reference);
    expect(svObject.getPreviousRevisions()).to.be.deep.equal(previousRevisions.slice(0, 2));
    expect(svObject.isDeleted()).to.be.false();
  });

  it('should delete SVObject if action is "delete"', async () => {
    const [previousSVObject] = getSVObjectsFixture();

    svObjectRepository.find.returns(previousSVObject);

    dpObject.setRevision(1);

    dpObject.setAction(DPObject.ACTIONS.DELETE);

    await updateSVObject(contractId, userId, reference, dpObject);

    expect(svObjectRepository.store).to.calledOnce();

    const svObject = svObjectRepository.store.getCall(0).args[0];

    expect(svObject).to.be.instanceOf(SVObject);
    expect(svObject.getUserId()).to.be.equal(userId);
    expect(svObject.getDPObject()).to.be.equal(dpObject);
    expect(svObject.getReference()).to.be.equal(reference);
    expect(svObject.getPreviousRevisions()).to.be.deep.equal([
      previousSVObject.getCurrentRevision(),
    ]);
    expect(svObject.isDeleted()).to.be.true();
  });


  it('should throw error if action is not supported', async () => {
    dpObject.setAction(100);

    let error;
    try {
      await updateSVObject(contractId, userId, reference, dpObject);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(Error);

    expect(svObjectRepository.store).not.to.be.called();
  });
});
