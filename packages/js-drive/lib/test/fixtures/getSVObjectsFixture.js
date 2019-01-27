const SVObject = require('../../stateView/object/SVObject');

const getDPObjectsFixture = require('./getDPObjectsFixture');
const getReferenceFixture = require('./getReferenceFixture');

/**
 * @return {SVObject[]}
 */
function getSVObjectsFixture() {
  const { userId } = getDPObjectsFixture;
  const dpObjects = getDPObjectsFixture();

  return dpObjects.map((dpObject, i) => new SVObject(
    userId,
    dpObject,
    getReferenceFixture(i + 1),
  ));
}

module.exports = getSVObjectsFixture;
