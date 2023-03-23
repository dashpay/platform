const crypto = require('node:crypto');

/**
 * @param {string} value
 * @returns {boolean|string}
 */
function validateTenderdashNodeKey(value) {
  if (value.length < 1) {
    return 'should not be empty';
  }

  const nodeKey = Buffer.from(value, 'base64');

  if (nodeKey.length !== 64) {
    return 'invalid format';
  }

  // TODO: Make it work
  // const privateKey = nodeKey.slice(0, 32);
  // const privateKeyDer = Buffer.concat([
  //   Buffer.from('302e020100300506032b657004220420', 'hex'), // Static value
  //   privateKey,
  // ]);
  // const privateKeyObject = crypto.createPrivateKey({
  //   format: 'der',
  //   type: 'pkcs8',
  //   key: privateKeyDer,
  // });
  //
  // const publicKey = nodeKey.slice(32);
  // const publicKeyDer = Buffer.concat([
  //   Buffer.from('302a300506032b6570032100', 'hex'), // Static value
  //   publicKey,
  // ]);
  // const publicKeyObject = crypto.createPublicKey({
  //   format: 'der',
  //   type: 'spki',
  //   key: publicKeyDer,
  // });
  //
  // const data = Buffer.from('Hello, world!');
  //
  // const signature = crypto.sign('sha256', data, privateKeyObject);
  //
  // const verify = crypto.verify('sha256', data, publicKeyObject, signature);
  //
  // if (!verify) {
  //   return 'malformed key';
  // }

  return true;
}

module.exports = validateTenderdashNodeKey;
