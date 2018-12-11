const DapObject = require('../../dapObject/DapObject');
const getLovelyDapContract = require('./getLovelyDapContract');

/**
 * @return {DapObject[]}
 */
module.exports = function getLovelyDapObjects() {
  const userId = '';
  const dapContract = getLovelyDapContract();

  return [
    new DapObject(dapContract, userId, 'niceObject', { name: 'Cutie' }),
    new DapObject(dapContract, userId, 'prettyObject', { lastName: 'Shiny' }),
  ];
};
