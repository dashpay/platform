const DPObjectFactory = require('@dashevo/dpp/lib/object/DPObjectFactory');

const getDPContractFixture = require('./getDPContractFixture');

const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

/**
 * @return {DPObject[]}
 */
function getDPObjectsFixture() {
  const dpContract = getDPContractFixture();

  const validateDPObjectStub = () => {};

  const factory = new DPObjectFactory(
    userId,
    dpContract,
    validateDPObjectStub,
  );

  return [
    factory.create('niceObject', { name: 'Cutie' }),
    factory.create('prettyObject', { lastName: 'Shiny' }),
    factory.create('prettyObject', { lastName: 'Sweety' }),
  ];
}

module.exports = getDPObjectsFixture;
module.exports.userId = userId;
