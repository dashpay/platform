## About Documents

## Fetching a document

Allow to fetch documents of a certains type specified by the schema contract provided. 

```js
const doc = await sdk.platform.documents.fetch(type,queryOpts);
```   

{string} - type : the type of document (example on dashPay : 'profile')  
{Object} - queryOpts:   
    - {Object} where - Mongo-like query  
    - {Object} orderBy - Mongo-like sort field  
    - {number} limit - how many objects to fetch  
    - {number} startAt - number of objects to skip  
    - {number} startAfter - exclusive skip  


## Creating a document

```js
const doc = sdk.platform.documents.create();
```   

## Broadcasting a document

```js
const txid = sdk.platform.documents.broadcast(doc);
```   

## Deleting a document

```js
const doc = sdk.platform.documents.delete();
```   

