**Usage**: `account.encode(method, data)`    
**Description**: Allow to encode any raw data   
**Notes**: Methods allowed right now limited to cbor (used by platform protocol).      

Parameters: 

| parameters        | type          | required       | Description                                      |  
|-------------------|---------------|----------------| -------------------------------------------------|
| **method**        | String        | yes            | Enter a valid encoding method (one of: ['cbor']) |
| **data**          | Object/String | yes            | A value to encode                                |

Returns : encoded value (Buffer)  

Example : 
```js
const jsonObject = {
    string: 'string',
    list: ['a', 'b', 'c', 'd'],
    obj: {
      int: 1,
      boolean: true,
      theNull: null,
    },
  };

const encodedJSON = account.encode('cbor', jsonObject)
console.log(Buffer.from(encodedJSON).toString('hex'));
```
