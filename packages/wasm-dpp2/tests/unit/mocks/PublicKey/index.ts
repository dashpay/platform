export const keyId = 2;
export const purpose = 'AUTHENTICATION';
export const securityLevel = 'CRITICAL';
export const keyType = 'ECDSA_SECP256K1';
export const binaryDataHex = '036a394312e40e81d928fde2bde7880070e4fa9c1d1d9b168da707ea468afa2b48';
export const binaryData = Buffer.from(binaryDataHex, 'hex');

export const keyIdSet = 3;
export const purposeSet = 'ENCRYPTION';
export const securityLevelSet = 'HIGH';
export const keyTypeSet = 'ECDSA_HASH160';
export const binaryDataSetHex = '0300000002e40e81d928fde2bde7880070e4fa9c1d1d9b168da707ea468afa2b48';
export const binaryDataSet = Buffer.from(binaryDataSetHex, 'hex');
