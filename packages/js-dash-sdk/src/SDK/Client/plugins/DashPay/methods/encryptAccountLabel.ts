import CryptoJS from "crypto-js"

/**
 *
 * @param sharedSecret
 * @param accountLabel
 * @param cipherIv
 */
export function encryptAccountLabel(
  sharedSecret: string,
  accountLabel:string = 'Default Account',
  cipherIv?: string
) {
  const labelBuffer = Buffer.from(accountLabel, 'utf8');
  const paddingBuffer = 31 - labelBuffer.length > 0 ? Buffer.alloc(31 - labelBuffer.length) : Buffer.alloc(0);

  const paddedLabelBuffer = Buffer.concat([paddingBuffer, labelBuffer]);

  const cipherSecret = {
    iv: cipherIv ? CryptoJS.enc.Hex.parse(cipherIv) : CryptoJS.lib.WordArray.random(16),
    padding: CryptoJS.pad.Pkcs7,
    mode: CryptoJS.mode.CBC,
  }

  const cipher = CryptoJS.AES.encrypt(
    paddedLabelBuffer.toString('utf8'),
    CryptoJS.enc.Hex.parse(sharedSecret),
    cipherSecret
  );

  const iv = cipher.iv.toString(CryptoJS.enc.Hex);
  const ciphertextBuffer = Buffer.from(cipher.ciphertext.toString(CryptoJS.enc.Hex), 'hex');
  const ivBuffer = Buffer.from(iv, 'hex');

  return Buffer.concat([ivBuffer, ciphertextBuffer]).toString('base64');
}
