const findDuplicatedDPObjects = require('../../../../lib/stPacket/validation/findDuplicatedDPObjects');

const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');

describe('findDuplicatedDPObjects', () => {
  let rawDPObjects;

  beforeEach(() => {
    rawDPObjects = getDPObjectsFixture().map(o => o.toJSON());
  });

  it('should return empty array if there are no duplicated DP Objects', () => {
    const result = findDuplicatedDPObjects(rawDPObjects);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(0);
  });

  it('should return duplicated DP Objects', () => {
    rawDPObjects.push(rawDPObjects[0]);

    const result = findDuplicatedDPObjects(rawDPObjects);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.be.equal(rawDPObjects[0]);
  });
});
