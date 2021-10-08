**Usage**: `keychain.getDIP15ExtendedKey(userUniqueId, contactUniqueId, index, accountIndex = 0, type = 'HDPrivateKey')`    

**Description**: Return a DIP15 Extended Key of a 2-contacts relationship

Parameters: 

| parameters          | type        | required                  | Description                                                    |   
|---------------------|-------------|---------------------------| ---------------------------------------------------------------|
| **userUniqueId**    | string      | yes                       | Current DashPay unique UserID                                  |
| **contactUniqueId** | string      | yes                       | Contact DashPay unique UserID                                  |
| **index**           | number      | no(=0)                    | the key index to derivate to                                   |
| **accountIndex**    | number      | no(=0)                    | the wallet account index from which to derivate                |
| **type**            | string      | no (default:HDPrivateKey) | type of returned keys. one of: ['HDPrivateKey','HDPublicKey']. |

Returns : {HDPrivateKey|HDPublicKey} (of path: `m/9'/5'/15'/accountIndex'/userId'/contactID'/index` on mainnet or `m/9'/1'/15'/...` on testnet)

Example: 
```js
// m/9'/5'/15'/0'/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0

const userUniqueId = '0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a';
const contactUniqueId = '0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5';

const DIP15ExtPrivKey_0 = keychain2.getDIP15ExtendedKey(userUniqueId, contactUniqueId, 0, 0, 'HDPrivateKey');
const { privateKey } = DIP15ExtPrivKey_0; //fac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60
```
