declare function _default(): Promise<{
    validatePublicKey(publicKeyBuffer: any): boolean;
    sign(data: any, key: any): Buffer;
    verifySignature(signature: any, data: any, publicKey: any): boolean;
}>;
export default _default;
