const getDPContractFixture = require('./getDPContractFixture');

const DPObjectFactory = require('../../object/DPObjectFactory');

const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

/**
 * @return {DPObject[]}
 */
module.exports = function getDPObjectsFixture() {
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
    factory.create('indexedObject', { firstName: 'William', lastName: 'Birkin' }),
    factory.create('indexedObject', { firstName: 'Leon', lastName: 'Kennedy' }),
  ];
};

module.exports.userId = userId;
