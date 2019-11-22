## About DPNS

DPNS is a special Dash Platform Application that is intended to provide a naming service for the Application Chain.  

Decoupling from the Blockchain identity allow to provide a unique user experience coupled with the highest security while remaining compatible with Identity Standard.


## Registering a new identity (User, Application ID)

Identity can be of multiple type, supported by this SDK, you will find these two types : 
   
- User Identity as type `user`

- Application Identity as type `application`


```js
const bob = sdk.platform.identities.register('user', {});
```   


## Searching an identity

```js
const bob = sdk.platform.identities.search('user', {});
```   

## Get an identity

```js
const bob = sdk.platform.identities.get('user', 'bob');
```  
