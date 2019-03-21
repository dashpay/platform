/* eslint no-unused-vars: "warn" */
/* eslint no-new: "warn" */
const spvService = require('../../services/spv');

const loadBloomFilter = filter => new Promise(() => spvService.loadBloomFilter(filter));

const addToBloomFilter = filter => new Promise(() => {
  spvService.addToBloomFilter(filter);
});
const clearBloomFilter = filter => new Promise(() => {
  spvService.clearBloomFilter(filter);
});
const getSpvData = filter => new Promise(() => {
  spvService.getSpvData(filter);
});
const findDataForBlock = (filter, blockHash) => new Promise(() => {
  spvService.findDataForBlock(filter, blockHash);
});

module.exports = {
  loadBloomFilter,
  addToBloomFilter,
  clearBloomFilter,
  getSpvData,
  findDataForBlock,
};
