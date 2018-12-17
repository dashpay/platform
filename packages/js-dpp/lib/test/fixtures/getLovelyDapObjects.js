const DapObject = require('../../dapObject/DapObject');
const getLovelyDapContract = require('./getLovelyDapContract');

const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

/**
 * @return {DapObject[]}
 */
module.exports = function getLovelyDapObjects() {
  const dapContract = getLovelyDapContract();

  return [
    new DapObject(dapContract, userId, 'niceObject', { name: 'Cutie' }),
    new DapObject(dapContract, userId, 'prettyObject', { lastName: 'Shiny' }),
    new DapObject(dapContract, userId, 'prettyObject', { lastName: 'Sweety' }),
  ];
};

module.exports.userId = userId;
