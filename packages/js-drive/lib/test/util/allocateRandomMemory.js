/**
 *
 * @param {number} sizeInBytes - size of memory to allocate
 * @returns {number[]} - result of the allocation - array filled with random doubles
 */
/* istanbul ignore next */
module.exports = function allocateRandomMemory(sizeInBytes) {
  // This constant is inside of this function because
  // it's easier to pass the whole function to the isolate in this case
  const NUMBER_SIZE_IN_BYTES = 64 / 8;
  const storage = [];
  while ((storage.length * NUMBER_SIZE_IN_BYTES) < sizeInBytes) {
    storage.push(Math.random());
  }
  return storage;
};
