## About Documents

## Fetching a document

Allow to fetch documents of a certains type specified by the schema contract provided. 

```js
const doc = await sdk.platform.documents.get('dashpay.profile',{});
```  

`fetch(type,queryOpts)`

- `type` {string} : the type of document (example with the application named dashpay : 'dashpay.profile')  
- `queryOpts` {Object}:   
- `where` {Object} - Mongo-like query  
- `orderBy` {Object} - Mongo-like sort field  
- `limit` {number} - how many objects to fetch  
- `startAt` {number} - number of objects to skip  
- `startAfter` {number} - exclusive skip  


## Creating a document

```js
const doc = sdk.platform.documents.create('dashpay.profile', identity, data);
```   

## Broadcasting a document

```js
const txid = sdk.platform.documents.broadcast(doc, identity);
```   

