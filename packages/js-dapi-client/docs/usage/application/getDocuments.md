**Usage**: `async client.getDocuments(contractId, type, options)`
**Description**: Fetch Documents from Drive

Parameters:

| parameters             | type               | required       | Description                                                                                             |
|------------------------|--------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **contractId**         | String             | yes            | A valid registered contractId |
| **type**               | String             | yes            | DAP object type to fetch (e.g: 'preorder' in DPNS)    |
| **options.where**      | Object             | yes            | Mongo-like query |
| **options.orderBy**    | Object             | yes            | Mongo-like sort field |
| **options.limit**      | Number             | yes            | Limit the number of object to fetch |
| **options.startAt**    | Number             | yes            | number of objects to skip |
| **options.startAfter** | Number             | yes            | exclusive skip |

Returns : Promise<Buffer[]>

