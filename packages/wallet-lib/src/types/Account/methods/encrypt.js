import AES from 'crypto-js/aes.js';

const encrypt = function encrypt(method, data, secret) {
  const str = typeof data === 'string' ? data : data.toString();
  switch (method) {
    default:
      return AES.encrypt(str, secret).toString();
  }
};
export default encrypt;