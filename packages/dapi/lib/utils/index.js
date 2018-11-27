const utils = {
  getCorrectedHash: (reversedHashObj) => {
    const clone = Buffer.alloc(32);
    reversedHashObj.copy(clone);
    return clone.reverse().toString('hex');
  },
};

module.exports = utils;

