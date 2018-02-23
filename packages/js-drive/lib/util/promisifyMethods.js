const util = require('util');

module.exports = function promisifyMethods(original, methods) {
  const newObject = {};

  methods.forEach((method) => {
    newObject[method] = util.promisify(original[method]).bind(original);
  });

  return newObject;
};
