## About DPNS

DPNS is a special Dash Platform Application that is intended to provide a naming service for the Application Chain.  

Decoupling from the Blockchain identity allow to provide a unique user experience coupled with the highest security while remaining compatible with Identity Standard.

Limitation : max length of 63 characters on charset `0-9`,`A-Z`(case insensitive), `-`.

Domain names are linked to an Identity.

## Registering a new identity (User, Application ID)

Identity can be of multiple type, supported by this SDK, you will find these two types : 
   
- User Identity as type `user`
- Application Identity as type `application`


```js
const alice = sdk.platform.names.register('alice', '3GegupTgRfdN9JMS8R6QXF3B2VbZtiw63eyudh1oMJAk');
```   


## Get a name identity

```js
const aliceIdentity = sdk.platform.names.get('alice'); // aliceIdentity: 3GegupTgRfdN9JMS8R6QXF3B2VbZtiw63eyudh1oMJAk
```  
