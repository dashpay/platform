export = blsPrivateKeyFactory;
/**
 * Create an instance of BlsPrivateKey
 *
 * @param {string|Buffer|Uint8Array|PrivateKey} privateKey string must be hex
 * @returns {Promise<PrivateKey>}
 */
declare function blsPrivateKeyFactory(
  privateKey: string | Buffer | Uint8Array | any
): Promise<any>;
