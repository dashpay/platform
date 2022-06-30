import CryptoJS from "crypto-js"

/**
 * Allow from an encryptedAccountLabel provided and the shared secret to decrypt
 * a account label
 * @param encryptedAccountLabel
 * @param sharedSecret
 */
export function decryptAccountLabel(encryptedAccountLabel, sharedSecret) {
  const encryptAccountLabelBuffer = Buffer.from(encryptedAccountLabel, 'base64');
  const parsedSharedSecret = CryptoJS.enc.Hex.parse(sharedSecret);

  const encryptedCipherParams = CryptoJS.lib.CipherParams.create({
    ciphertext: CryptoJS.enc.Hex
      .parse(encryptAccountLabelBuffer.slice(16, encryptAccountLabelBuffer.length)
        .toString('hex')),
  });

  const parsedEncryptedAccountLabelBuffer = CryptoJS.enc.Hex
    .parse(encryptAccountLabelBuffer.slice(0, 16)
      .toString('hex'));

  const decryptedAccountLabel = CryptoJS.AES.decrypt(
    encryptedCipherParams,
    parsedSharedSecret,
    {
      iv: parsedEncryptedAccountLabelBuffer
    }
  );
  // Transform to UTF8 and unpad before returning.
  return decryptedAccountLabel
    .toString(CryptoJS.enc.Utf8)
    .replace(/\x00/g, '');

}
