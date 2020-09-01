const AES = require('crypto-js/aes');

const encrypt = function encrypt(method, data, secret) {
  const str = typeof data === 'string' ? data : data.toString();
  switch (method) {
    default:
      return AES.encrypt(str, secret).toString();
  }
};
module.exports = encrypt;
