const SVObject = require('../../../../lib/stateView/object/SVObject');

const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('SVObject', () => {
  let svObject;
  let userId;
  let dpObject;
  let reference;
  let isDeleted;
  let previousRevisions;

  beforeEach(() => {
    ({ userId } = getDPObjectsFixture);
    [dpObject] = getDPObjectsFixture();
    reference = getReferenceFixture();
    isDeleted = false;
    previousRevisions = [];

    svObject = new SVObject(
      userId,
      dpObject,
      reference,
      isDeleted,
      previousRevisions,
    );
  });

  describe('#getUserId', () => {
    it('should return user ID', () => {
      const result = svObject.getUserId();

      expect(result).to.equal(userId);
    });
  });

  describe('#getDPObject', () => {
    it('should return DP Object', () => {
      const result = svObject.getDPObject();

      expect(result.toJSON()).to.deep.equal(dpObject.toJSON());
    });
  });

  describe('#markAsDeleted', () => {
    it('should mark object as deleted', () => {
      const result = svObject.markAsDeleted();

      expect(result).to.equal(svObject);

      expect(svObject.deleted).to.be.true();
    });
  });

  describe('#isDeleted', () => {
    it('should return true if object is deleted', () => {
      const result = svObject.isDeleted();

      expect(result).to.be.false();
    });
  });

  describe('#toJSON', () => {
    it('should return SVObject as a plain object', () => {
      const result = svObject.toJSON();

      expect(result).to.deep.equal({
        userId,
        dpObject: dpObject.toJSON(),
        reference: reference.toJSON(),
        isDeleted,
        previousRevisions,
      });
    });
  });
});
