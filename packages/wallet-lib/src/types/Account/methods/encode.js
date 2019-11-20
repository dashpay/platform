const cbor = require('cbor');

module.exports = (method, data) => {
  const str = data === 'string' ? data : data.toString();
  switch (method) {
    default: // cbor
      return cbor.encode(str);
  }
};
