/* eslint no-unused-vars: "warn" */
/* eslint no-new: "warn" */
const spvService = require('../../services/spv');

const loadBloomFilter = filter => new Promise(() => spvService.loadBloomFilter(filter));

const addToBloomFilter = filter => // spvService.addToBloomFilter is not a function
  new Promise(() => {
    spvService.addToBloomFilter(filter);
  });
const clearBloomFilter = filter => // spvService.clearBloomFilter is not a function
  new Promise(() => {
    spvService.clearBloomFilter(filter);
  });
const getSpvData = filter => // spvService.getSpvData is not a function
  new Promise(() => {
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
