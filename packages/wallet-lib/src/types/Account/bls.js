const bls = require('bls-lib');

const getPublicKey = (data) => new Promise(
  (resolve, reject) => bls.onModuleInit(() => {
    try {
      bls.init();

      const sec = bls.secretKey();
      const pub = bls.publicKey();
      const sig = bls.signature();

      bls.secretKeySetByCSPRNG(sec);
      bls.sign(sig, sec, data);

      const publicKey = bls.getPublicKey(pub, sec);

      resolve(publicKey);
    } catch (err) {
      reject(err);
    }
  }),
);

module.exports = {
  bls,
  getPublicKey,
};
