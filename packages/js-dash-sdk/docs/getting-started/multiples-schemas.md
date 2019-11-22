# Working with multiple schemas

As you saw in the quick start example, we initiated our SDK using `opts.schema` and passing a single schema as reference.  
You also saw that we required a direct property `profile` without anything more specified.  

But during your exploration and developements, you might need to work with multiples schemas, the SDK has got you covered. 
Instead of a single schema, pass it a named object referencing those `opts.schemas` and access using a dot syntax. 

If both opts.schemas and .schema are passed, the later is discarded.

```js
const opts = {
  schemas: {
    dashpay: dashpayJsonSchema,
    dashkeys: dashkeysJsonSchema
  }
};
const bobProfile = await sdk.platform.fetchDocuments('dashpay.profile', {name:'bob'})
const bobMsgKey = await sdk.platform.fetchDocuments('dashkeys.key', {name:'bob'});
```
