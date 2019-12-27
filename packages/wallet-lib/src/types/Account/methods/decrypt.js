const CryptoJS = require('crypto-js');
const { AES } = CryptoJS;

const decrypt = function (method, data, secret, encoding = '') {
  let decrypted;
  switch (method) {
    default:
      decrypted = AES.decrypt(data, secret);
      return (encoding === 'hex') ? decrypted.toString(CryptoJS.enc.Hex) : decrypted.toString(CryptoJS.enc.Utf8);
  }
};
module.exports = decrypt;
