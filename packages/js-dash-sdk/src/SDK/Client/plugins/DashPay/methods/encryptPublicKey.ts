import CryptoJS from "crypto-js"
/**
 * @param publicKeyBuffer - public key in buffer format
 * @param encryptedSharedSecret - shared secret in hexadecimal format
 * @returns {null}
 */
export function encryptPublicKey(publicKeyBuffer, encryptedSharedSecret) {
  if(!Buffer.isBuffer(publicKeyBuffer)){
    throw new Error("Expected publicKey as Buffer");
  }
  const parsedPublicKey = CryptoJS.enc.Hex.parse(publicKeyBuffer.toString('hex'));
  const parsedEncryptedSharedSecret = CryptoJS.enc.Hex.parse(encryptedSharedSecret);

  const cipher = CryptoJS.AES.encrypt(parsedPublicKey, parsedEncryptedSharedSecret, {
    iv: CryptoJS.lib.WordArray.random(16),
    padding: CryptoJS.pad.Pkcs7,
    mode: CryptoJS.mode.CBC,
  });
  const cipherText = cipher.ciphertext.toString(CryptoJS.enc.Hex);
  if (!Buffer.from(cipherText, 'hex').length) {
    throw new Error('Invalid cipher size');
  }
  const ivText = cipher.iv.toString(CryptoJS.enc.Hex);

  return Buffer.concat([
    Buffer.from(ivText, 'hex'),
    Buffer.from(cipherText, 'hex'),
  ]).toString('hex');
};
