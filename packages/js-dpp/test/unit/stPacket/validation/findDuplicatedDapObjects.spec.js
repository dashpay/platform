const findDuplicatedDapObjects = require('../../../../lib/stPacket/validation/findDuplicatedDapObjects');

const getDapObjectsFixture = require('../../../../lib/test/fixtures/getDapObjectsFixture');

describe('findDuplicatedDapObjects', () => {
  let rawDapObjects;

  beforeEach(() => {
    rawDapObjects = getDapObjectsFixture().map(o => o.toJSON());
  });

  it('should return empty array if there are no duplicated DAP Objects', () => {
    const result = findDuplicatedDapObjects(rawDapObjects);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(0);
  });

  it('should return duplicated DAP Objects', () => {
    rawDapObjects.push(rawDapObjects[0]);

    const result = findDuplicatedDapObjects(rawDapObjects);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.be.equal(rawDapObjects[0]);
  });
});
