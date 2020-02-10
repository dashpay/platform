**Usage**: `new KeyChain(opts)`  
**Description**: This method create a new KeyChain. Which handle handle the derivation and handling of the HDRootKey (when init from an HDPrivKey).  

While both the seed and the mnemonic would allow to generate other coins private keys, a HDRootKey is specific to a coin, which is why it's the value used in store..    

Parameters: 

| parameters                         | type            | required       | Description                                                                                                                                                                    |  
|------------------------------------|-----------------|----------------| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **opts.network**                   | Network|String  | no (testnet)   | The network to use for the KeyChain address derivation                                                          |
| **opts.type**                      | string          | yes            | The type of the KeyChain (HDPrivateKey, HDPublicKey or privateKey) |
| **opts.HDPrivateKey**              | object          | yes (if type)  | If type is HDPrivateKey, the root HDPrivateKey to allow KeyChain to generate new address |
| **opts.HDPublicKey**               | object          | yes (if type)  | If type is HDPublicKey, the root HDPublicKey to allow KeyChain to generate new public address |
| **opts.privateKey**                | object          | yes (if type)  | If type is a PrivateKey, the PrivKey to allow KeyChain to manage public address |
| **opts.keys**                      | object          | no             | If required, allow to create KeyChain by passing it a set of keys  |

Returns : Keychain instance.

