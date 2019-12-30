## About Identities

Identities are blockchain-based identitiers for individuals (user) and applications.  
Identity is the atomic elements that linked with additional applications can be extended toward providing new functionnalities.   

Read more on [DPNS Name Service](/platform/names) that leverages Identity to allow you to create name-based user experiences.

## Registering a new identity (User, Application ID)

Identity can be of multiple type, supported by this SDK, you will find these two types : 
   
- User Identity as type `USER`
- Application Identity as type `APPLICATION`

```js
// Assuming first user id created on our third HDWallet account (index:2)
const alice = sdk.platform.identities.register('user');

// Assuming first application id created on our third HDWallet account (index:2).
const appId = sdk.platform.identities.register('application');
```   

## Get an identity

```js
const bob = sdk.platform.identities.get('3GegupTgRfdN9JMS8R6QXF3B2VbZtiw63eyudh1oMJAk');
```  
